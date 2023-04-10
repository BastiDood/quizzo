use core::fmt::{self, Display};

pub enum Error {
    BadInput,
    UnknownQuiz,
    UnknownUser,
    Fatal,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BadInput => "Unacceptable input.",
            Self::UnknownQuiz => "Quiz not found. It may have already expired.",
            Self::UnknownUser => "Unknown user.",
            Self::Fatal => "Oops! We have encountered an unrecoverable error on our end.",
        })
    }
}

pub type Result<T> = core::result::Result<T, Error>;
