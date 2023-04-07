#![no_std]

pub mod error;

use core::num::NonZeroU32;
use model::Quiz;
use tokio_postgres::error::SqlState;

pub use model::Uuid;
pub use tokio_postgres::{tls::NoTls, Client, Config};

pub struct Database(Client);

impl From<Client> for Database {
    fn from(client: Client) -> Self {
        Self(client)
    }
}

fn deserialize_from_row(row: tokio_postgres::Row) -> Result<Quiz, tokio_postgres::Error> {
    let timeout = row.try_get(3)?;
    let answer = row.try_get(2)?;
    let question = row.try_get(0)?;
    let choices = row.try_get(1)?;
    Ok(Quiz { question, choices, answer, timeout })
}

impl Database {
    pub async fn init_quiz(&self, question: &str, timeout: u32) -> error::Result<Uuid> {
        let err = match self
            .0
            .query_opt("INSERT INTO quiz (question, timeout) VALUES ($1, $2) RETURNING id", &[&question, &timeout])
            .await
        {
            Ok(row) => {
                let uuid = row.ok_or(error::Error::Fatal)?.try_get(0).map_err(|_| error::Error::Fatal)?;
                return Ok(uuid);
            }
            Err(err) => err,
        };

        let err = err.as_db_error().ok_or(error::Error::Fatal)?;
        if *err.code() != SqlState::CHECK_VIOLATION {
            return Err(error::Error::Fatal);
        }

        let constraint = err.constraint().ok_or(error::Error::Fatal)?;
        if constraint != "quiz_timeout_check" {
            return Err(error::Error::Fatal);
        }

        Err(error::Error::BadInput)
    }

    pub async fn get_quiz(&self, id: Uuid) -> error::Result<Quiz> {
        let row = self
            .0
            .query_opt("SELECT question, choices, answer, timeout FROM quiz WHERE id = $1", &[&id])
            .await
            .map_err(|_| error::Error::Fatal)?
            .ok_or(error::Error::NotFound)?;
        deserialize_from_row(row).map_err(|_| error::Error::Fatal)
    }

    pub async fn pop_quiz(&self, id: Uuid) -> error::Result<Quiz> {
        let row = self
            .0
            .query_opt("DELETE FROM quiz WHERE id = $1 RETURNING question, choices, answer, timeout", &[&id])
            .await
            .map_err(|_| error::Error::Fatal)?
            .ok_or(error::Error::NotFound)?;
        deserialize_from_row(row).map_err(|_| error::Error::Fatal)
    }

    pub async fn add_choice(&self, id: Uuid, choice: &str) -> error::Result<()> {
        let err = match self
            .0
            .execute("UPDATE quiz SET choices = array_append(choices, $2) WHERE id = $1", &[&id, &choice])
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

    pub async fn remove_choice(&self, id: Uuid, index: NonZeroU32) -> error::Result<()> {
        let index = index.get();
        match self
            .0
            .execute(
                "UPDATE quiz SET answer = DEFAULT, choices = choices[1:$2-1] || choices[$2+1:NULL] WHERE id = $1",
                &[&id, &index],
            )
            .await
        {
            Ok(1) => Ok(()),
            Ok(0) => Err(error::Error::NotFound),
            _ => Err(error::Error::Fatal),
        }
    }

    pub async fn set_answer(&self, id: Uuid, answer: u32) -> error::Result<()> {
        let err = match self.0.execute("UPDATE quiz SET answer = $2 WHERE id = $1", &[&id, &answer]).await {
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
