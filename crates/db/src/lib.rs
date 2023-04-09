#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod error;

use alloc::boxed::Box;
use core::num::{NonZeroI16, NonZeroU64};
use tokio_postgres::error::SqlState;

pub use futures_util::{TryStream, TryStreamExt};
pub use model::{Quiz, RawQuiz};
pub use tokio_postgres::{tls::NoTls, Client, Config};

pub struct Database(Client);

impl From<Client> for Database {
    fn from(client: Client) -> Self {
        Self(client)
    }
}

fn deserialize_raw_quiz_from_row(row: tokio_postgres::Row) -> Result<RawQuiz, tokio_postgres::Error> {
    let expiration = row.try_get("expiration")?;
    let answer = row.try_get("answer")?;
    let question = row.try_get("question")?;
    let choices = row.try_get("choices")?;
    Ok(RawQuiz { question, choices, answer, expiration })
}

fn deserialize_quiz_from_row(row: tokio_postgres::Row) -> error::Result<Quiz> {
    let id: i16 = row.try_get("id").map_err(|_| error::Error::Fatal)?;
    let id = NonZeroI16::new(id).ok_or(error::Error::Fatal)?;
    let raw = deserialize_raw_quiz_from_row(row).map_err(|_| error::Error::Fatal)?;
    Ok(Quiz { id, raw })
}

impl Database {
    pub async fn init_quiz(&self, user: NonZeroU64, question: &str) -> error::Result<NonZeroI16> {
        let uid = user.get() as i64;
        let err = match self
            .0
            .query_opt("INSERT INTO quiz (author, question) VALUES ($1, $2) RETURNING id", &[&uid, &question])
            .await
        {
            Ok(row) => {
                let row = row.ok_or(error::Error::Fatal)?;
                let id: i16 = row.try_get("id").map_err(|_| error::Error::Fatal)?;
                return NonZeroI16::new(id).ok_or(error::Error::Fatal);
            }
            Err(err) => err,
        };

        let err = err.as_db_error().ok_or(error::Error::Fatal)?;
        if *err.code() != SqlState::CHECK_VIOLATION {
            return Err(error::Error::Fatal);
        }

        let constraint = err.constraint().ok_or(error::Error::Fatal)?;
        if constraint == "quiz_question_check" {
            return Err(error::Error::BadInput);
        }

        Err(error::Error::Fatal)
    }

    pub async fn get_quiz(&self, user: NonZeroU64, quiz: NonZeroI16) -> error::Result<RawQuiz> {
        let uid = user.get() as i64;
        let qid = quiz.get();
        let row = self
            .0
            .query_opt(
                "SELECT question, choices, answer, expiration FROM quiz WHERE author = $1 AND id = $2",
                &[&uid, &qid],
            )
            .await
            .map_err(|_| error::Error::Fatal)?
            .ok_or(error::Error::NotFound)?;
        deserialize_raw_quiz_from_row(row).map_err(|_| error::Error::Fatal)
    }

    pub async fn get_quizzes_by_user(
        &self,
        user: NonZeroU64,
    ) -> error::Result<impl TryStream<Ok = Quiz, Error = error::Error> + '_> {
        let uid = user.get() as i64;
        Ok(self
            .0
            .query_raw("SELECT id, question, choices, answer, expiration FROM quiz WHERE author = $1", &[&uid])
            .await
            .map_err(|_| error::Error::Fatal)?
            .map_err(|_| error::Error::Fatal)
            .and_then(|row| core::future::ready(deserialize_quiz_from_row(row))))
    }

    pub async fn pop_quiz(&self, user: NonZeroU64, quiz: NonZeroI16) -> error::Result<RawQuiz> {
        let uid = user.get() as i64;
        let qid = quiz.get();
        let row = self
            .0
            .query_opt(
                "DELETE FROM quiz WHERE author = $1 AND id = $2 AND answer IS NOT NULL RETURNING question, choices, answer, expiration",
                &[&uid, &qid],
            )
            .await
            .map_err(|_| error::Error::Fatal)?
            .ok_or(error::Error::NotFound)?;
        deserialize_raw_quiz_from_row(row).map_err(|_| error::Error::Fatal)
    }

    pub async fn add_choice(&self, user: NonZeroU64, quiz: NonZeroI16, choice: &str) -> error::Result<()> {
        let uid = user.get() as i64;
        let qid = quiz.get();
        let err = match self
            .0
            .execute(
                "UPDATE quiz SET choices = array_append(choices, $3) WHERE author = $1 AND id = $2",
                &[&uid, &qid, &choice],
            )
            .await
        {
            Ok(1) => return Ok(()),
            Ok(0) => return Err(error::Error::NotFound),
            Err(err) => err,
            _ => return Err(error::Error::Fatal),
        };

        let err = err.as_db_error().ok_or(error::Error::Fatal)?;
        let constraint = err.constraint().ok_or(error::Error::Fatal)?;
        Err(match (err.code(), constraint) {
            // We tried to append too many values to the array.
            (&SqlState::CHECK_VIOLATION, "quiz_choices_length_check") => error::Error::TooMany,
            // We tried to append a string that is too long for the `VARCHAR`.
            (&SqlState::STRING_DATA_RIGHT_TRUNCATION, "quiz_choices_check") => error::Error::BadInput,
            // Unexpected error type.
            _ => error::Error::Fatal,
        })
    }

    pub async fn remove_choice(&self, user: NonZeroU64, quiz: NonZeroI16, index: u32) -> error::Result<Box<str>> {
        let index = i32::try_from(index).map_err(|_| error::Error::BadInput)?;
        let uid = user.get() as i64;
        let qid = quiz.get();
        let row = self
            .0
            .query_opt(
                "WITH old AS (SELECT * FROM quiz WHERE author = $1 AND id = $2) \
                 UPDATE quiz SET answer = DEFAULT, choices = quiz.choices[1:$3] || quiz.choices[$3+2:] \
                 FROM old \
                 WHERE quiz.author = old.author AND quiz.id = old.id \
                 RETURNING old.choices[$3+1] AS choice",
                &[&uid, &qid, &index],
            )
            .await
            .map_err(|_| error::Error::Fatal)?
            .ok_or(error::Error::NotFound)?;
        let choice: Box<str> = row.try_get("choice").map_err(|_| error::Error::Fatal)?;
        Ok(choice)
    }

    pub async fn set_question(&self, user: NonZeroU64, quiz: NonZeroI16, question: &str) -> error::Result<()> {
        let uid = user.get() as i64;
        let qid = quiz.get();
        let err = match self
            .0
            .execute("UPDATE quiz SET question = $3 WHERE author = $1 AND id = $2", &[&uid, &qid, &question])
            .await
        {
            Ok(1) => return Ok(()),
            Ok(0) => return Err(error::Error::NotFound),
            Err(err) => err,
            _ => return Err(error::Error::Fatal),
        };

        let err = err.as_db_error().ok_or(error::Error::Fatal)?;
        if *err.code() != SqlState::CHECK_VIOLATION {
            return Err(error::Error::Fatal);
        }

        let constraint = err.constraint().ok_or(error::Error::Fatal)?;
        if constraint != "question_check" {
            return Err(error::Error::Fatal);
        }

        Err(error::Error::BadInput)
    }

    pub async fn set_answer(&self, user: NonZeroU64, quiz: NonZeroI16, answer: u16) -> error::Result<()> {
        let answer = i16::try_from(answer).map_err(|_| error::Error::BadInput)?;
        let uid = user.get() as i64;
        let qid = quiz.get();
        let err = match self
            .0
            .execute("UPDATE quiz SET answer = $3 WHERE author = $1 AND id = $2", &[&uid, &qid, &answer])
            .await
        {
            Ok(1) => return Ok(()),
            Ok(0) => return Err(error::Error::NotFound),
            Err(err) => err,
            _ => return Err(error::Error::Fatal),
        };

        let err = err.as_db_error().ok_or(error::Error::Fatal)?;
        if *err.code() != SqlState::CHECK_VIOLATION {
            return Err(error::Error::Fatal);
        }

        let constraint = err.constraint().ok_or(error::Error::Fatal)?;
        if constraint != "answer_check" {
            return Err(error::Error::Fatal);
        }

        Err(error::Error::BadInput)
    }

    pub async fn set_expiration(&self, user: NonZeroU64, quiz: NonZeroI16, expiration: u16) -> error::Result<()> {
        let expiration = i16::try_from(expiration).map_err(|_| error::Error::BadInput)?;
        let uid = user.get() as i64;
        let qid = quiz.get();
        let err = match self
            .0
            .execute("UPDATE quiz SET expiration = $3 WHERE author = $1 AND id = $2", &[&uid, &qid, &expiration])
            .await
        {
            Ok(1) => return Ok(()),
            Ok(0) => return Err(error::Error::NotFound),
            Err(err) => err,
            _ => return Err(error::Error::Fatal),
        };

        let err = err.as_db_error().ok_or(error::Error::Fatal)?;
        if *err.code() != SqlState::CHECK_VIOLATION {
            return Err(error::Error::Fatal);
        }

        let constraint = err.constraint().ok_or(error::Error::Fatal)?;
        if constraint != "expiration_check" {
            return Err(error::Error::Fatal);
        }

        Err(error::Error::BadInput)
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Database, NoTls, NonZeroU64, Quiz, TryStreamExt};

    #[tokio::test(flavor = "current_thread")]
    async fn database_test() {
        use std::env::var;
        let user = var("PG_USERNAME").unwrap();
        let pass = var("PG_PASSWORD").unwrap();
        let host = var("PG_HOSTNAME").unwrap();
        let data = var("PG_DATABASE").unwrap();

        // Dummy credentials for the database
        let (client, conn) = Config::new()
            .user(&user)
            .password(&pass)
            .host(&host)
            .dbname(&data)
            .port(5432)
            .connect(NoTls)
            .await
            .expect("cannot connect to database");
        let handle = tokio::spawn(conn);
        let db = Database::from(client);

        // Quiz creation
        let uid = NonZeroU64::new(10).unwrap();
        let qid = db.init_quiz(uid, "Hello world?").await.unwrap();

        // Initial quiz retrieval
        let init = db.get_quiz(uid, qid).await.unwrap();
        assert_eq!(init.question, "Hello world?");
        assert!(init.answer.is_none());
        assert_eq!(init.expiration, 10);
        assert!(init.choices.is_empty());

        // Get all quizzes from the user
        let quizzes: Vec<_> = db.get_quizzes_by_user(uid).await.unwrap().try_collect().await.unwrap();
        assert_eq!(quizzes.as_slice(), &[Quiz { id: qid, raw: init }]);
        drop(quizzes);

        // Set new quiz parameters
        db.set_question(uid, qid, "What is the largest planet in the solar system?").await.unwrap();
        db.set_expiration(uid, qid, 50).await.unwrap();

        // Add new choices
        db.add_choice(uid, qid, "Mercury").await.unwrap();
        db.add_choice(uid, qid, "Venus").await.unwrap();
        db.add_choice(uid, qid, "Earth").await.unwrap();
        db.add_choice(uid, qid, "Titan").await.unwrap();
        db.add_choice(uid, qid, "Mars").await.unwrap();
        db.add_choice(uid, qid, "Jupiter").await.unwrap();
        db.add_choice(uid, qid, "Saturn").await.unwrap();
        db.add_choice(uid, qid, "Ganymede").await.unwrap();
        db.add_choice(uid, qid, "Uranus").await.unwrap();
        db.add_choice(uid, qid, "Neptune").await.unwrap();
        db.add_choice(uid, qid, "Orion").await.unwrap();
        db.add_choice(uid, qid, "Pluto").await.unwrap();

        // Remove invalid choices
        assert_eq!(db.remove_choice(uid, qid, 3).await.unwrap().as_ref(), "Titan");
        assert_eq!(db.remove_choice(uid, qid, 6).await.unwrap().as_ref(), "Ganymede");
        assert_eq!(db.remove_choice(uid, qid, 8).await.unwrap().as_ref(), "Orion");

        // Set a new answer
        db.set_answer(uid, qid, 4).await.unwrap();

        // Pop the answer off
        let quiz = db.pop_quiz(uid, qid).await.unwrap();
        assert_eq!(quiz.question, "What is the largest planet in the solar system?");
        assert_eq!(quiz.answer.unwrap(), 4);
        assert_eq!(quiz.expiration, 50);
        assert_eq!(
            quiz.choices.as_slice(),
            vec!["Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune", "Pluto"]
        );

        // Verify that the quiz has been removed
        let quizzes: Vec<_> = db.get_quizzes_by_user(uid).await.unwrap().try_collect().await.unwrap();
        assert!(quizzes.is_empty());

        drop(db);
        handle.await.unwrap().unwrap();
    }
}
