#![no_std]

pub mod error;

pub use model::Uuid;
pub use tokio_postgres::{tls::NoTls, Client, Config};

pub type Result<T> = core::result::Result<T, tokio_postgres::error::Error>;

pub struct Database(Client);

impl From<Client> for Database {
    fn from(client: Client) -> Self {
        Self(client)
    }
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
        assert_eq!(*err.code(), tokio_postgres::error::SqlState::CHECK_VIOLATION);

        let constraint = err.constraint().ok_or(error::Error::Fatal)?;
        assert_eq!(constraint, "timeout_check");

        Err(error::Error::BadInput)
    }

    pub async fn set_answer(&self, id: Uuid, answer: u32) -> error::Result<()> {
        let err = match self.0.execute("UPDATE quiz SET answer = $2 WHERE id = $1", &[&id, &answer]).await {
            Ok(1) => return Ok(()),
            Ok(0) => return Err(error::Error::NotFound),
            Err(err) => err,
            _ => unreachable!(),
        };

        let err = err.as_db_error().ok_or(error::Error::Fatal)?;
        assert_eq!(*err.code(), tokio_postgres::error::SqlState::CHECK_VIOLATION);

        let constraint = err.constraint().ok_or(error::Error::Fatal)?;
        assert_eq!(constraint, "answer_check");

        Err(error::Error::BadInput)
    }
}
