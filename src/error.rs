use serde_json::error::Category;
use std::fmt::{self, Display};

pub enum Error {
    UnsupportedInteraction,
    UnknownCommandId,
    UnknownCommandName,
    InvalidParams,
    UnknownParamName,
    InvalidUri,
    /// HTTP fetch error.
    FailedFetch,
    /// JSON syntax error detected.
    Syntax,
    /// Unexpected JSON data types encountered.
    Data,
}

impl From<hyper::Error> for Error {
    fn from(_: hyper::Error) -> Self {
        Self::FailedFetch
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        match err.classify() {
            Category::Data => Self::Data,
            Category::Syntax => Self::Syntax,
            _ => Self::FailedFetch,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Error::*;
        f.write_str(match self {
            UnsupportedInteraction => "Unsupported interaction.",
            UnknownCommandId => "Unknown command ID.",
            UnknownCommandName => "Unknown command name.",
            InvalidParams => "Invalid parameter list.",
            UnknownParamName => "Unknown parameter name.",
            InvalidUri => "Invalid URI.",
            FailedFetch => "Failed to fetch the JSON data.",
            Syntax => "Syntax error in JSON detected.",
            Data => "Unexpected data types in JSON detected.",
        })
    }
}

pub type Result<T> = core::result::Result<T, Error>;
