use core::fmt::{self, Display};

pub enum Error {
    UnsupportedInteraction,
    InvalidParams,
    UnknownQuiz,
    UnknownUser,
    UnknownCommandName,
    Unrecoverable,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;
        f.write_str(match self {
            UnsupportedInteraction => "Unsupported interaction.",
            UnknownQuiz => "Quiz not found. It may have already expired.",
            UnknownUser => "Unknown user.",
            UnknownCommandName => "Unknown command name.",
            InvalidParams => "Invalid parameter list.",
            Unrecoverable => "Oops! We have encountered an unrecoverable error on our end.",
        })
    }
}

pub type Result<T> = core::result::Result<T, Error>;
