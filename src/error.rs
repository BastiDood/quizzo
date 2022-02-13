use std::fmt::{self, Display};

pub enum Error {
    UnsupportedInteraction,
    UnknownCommandId,
    UnknownCommandName,
    InvalidParams,
    UnknownParamName,
    InvalidUri,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::UnsupportedInteraction => "Unsupported interaction.",
            Error::UnknownCommandId => "Unknown command ID.",
            Error::UnknownCommandName => "Unknown command name.",
            Error::InvalidParams => "Invalid parameter list.",
            Error::UnknownParamName => "Unknown parameter name.",
            Error::InvalidUri => "Invalid URI.",
        })
    }
}

pub type Result<T> = core::result::Result<T, Error>;
