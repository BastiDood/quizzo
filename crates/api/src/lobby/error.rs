use core::fmt::{self, Display};

pub enum Error {
    UnsupportedInteraction,
    InvalidParams,
    UnknownQuiz,
    UnknownUser,
    UnknownCommandName,
    Fatal,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnsupportedInteraction => "Unsupported interaction.",
            Self::UnknownQuiz => "Quiz not found. It may have already expired.",
            Self::UnknownUser => "Unknown user.",
            Self::UnknownCommandName => "Unknown command name.",
            Self::InvalidParams => "Invalid parameter list.",
            Self::Fatal => "Oops! We have encountered an unrecoverable error on our end.",
        })
    }
}

pub type Result<T> = core::result::Result<T, Error>;
