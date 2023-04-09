#![no_std]
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
            .query_opt("INSERT INTO quiz (user, question) VALUES ($1, $2) RETURNING id", &[&uid, &question])
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
                "DELETE FROM quiz WHERE author = $1 AND id = $2 AND answer IS NOT NULL RETURNING question, choices, answer, timeout",
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

    pub async fn remove_choice(&self, user: NonZeroU64, quiz: NonZeroI16, index: u16) -> error::Result<Box<str>> {
        let uid = user.get() as i64;
        let qid = quiz.get();
        let index = i16::try_from(index).map_err(|_| error::Error::BadInput)?;
        let row = self
            .0
            .query_opt(
                "UPDATE quiz SET answer = DEFAULT, choices = choices[1:$3] || choices[$3+2:] WHERE author = $1 AND id = $2 RETURNING choices[$3+1] AS choice",
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

    pub async fn set_answer(&self, user: NonZeroU64, quiz: NonZeroI16, answer: u32) -> error::Result<()> {
        let uid = user.get() as i64;
        let qid = quiz.get();
        let err =
            match self.0.execute("UPDATE quiz SET answer = $3 WHERE author = $1 id = $2", &[&uid, &qid, &answer]).await
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
}
