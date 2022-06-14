use mongodb::error::{ErrorKind, WriteError, WriteFailure};

#[derive(Debug)]
pub enum Error {
    /// The object we are trying to insert already exists.
    AlreadyExists,
    /// Arithmetic overflow occurred when computing the expiration date.
    TimeOverflow,
    /// Unrecoverable error.
    Fatal,
}

impl From<ErrorKind> for Error {
    fn from(err: ErrorKind) -> Self {
        if let ErrorKind::Write(WriteFailure::WriteError(WriteError { code: 11000, .. })) = err {
            Self::AlreadyExists
        } else {
            Self::Fatal
        }
    }
}

impl From<mongodb::error::Error> for Error {
    fn from(err: mongodb::error::Error) -> Self {
        (*err.kind).into()
    }
}

pub type Result<T> = core::result::Result<T, Error>;
