use core::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    BadInput,
    NotFound,
    Schema,
    Database,
    Dead,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BadInput => "Unacceptable input.",
            Self::NotFound => "Resource not found.",
            Self::Schema => "Discord provided an unexpected interaction schema.",
            Self::Database => "We encountered an unexpected database error on our end.",
            Self::Dead => "Oops! We encountered a logic error on our end. This is a bug.",
        })
    }
}

pub type Result<T> = core::result::Result<T, Error>;
