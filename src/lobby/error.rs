use hyper::http::{header::ToStrError, uri::InvalidUri};
use serde_json::error::Category;
use std::{
    fmt::{self, Display},
    num::ParseIntError,
};

pub enum Error {
    UnsupportedInteraction,
    UnknownUser,
    UnknownCommandName,
    InvalidParams,
    UnknownParamName,
    InvalidUri,
    /// HTTP fetch error.
    FailedFetch,
    /// JSON syntax error detected.
    Syntax,
    /// Unexpected data types encountered.
    Data,
    /// JSON payload too large.
    TooLarge,
    /// Payload is not JSON.
    UnknownContent,
    Unrecoverable,
}

impl From<ParseIntError> for Error {
    fn from(_: ParseIntError) -> Self {
        Self::Data
    }
}

impl From<hyper::Error> for Error {
    fn from(_: hyper::Error) -> Self {
        Self::FailedFetch
    }
}

impl From<ToStrError> for Error {
    fn from(_: ToStrError) -> Self {
        Self::Data
    }
}

impl From<InvalidUri> for Error {
    fn from(_: InvalidUri) -> Self {
        Self::InvalidUri
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
            UnknownUser => "Unknown user.",
            UnknownCommandName => "Unknown command name.",
            InvalidParams => "Invalid parameter list.",
            UnknownParamName => "Unknown parameter name.",
            InvalidUri => "Invalid URI.",
            FailedFetch => "Failed to fetch the JSON data.",
            Syntax => "Syntax error in JSON detected.",
            Data => "Unexpected data types detected.",
            TooLarge => "JSON payload is too large. Try sending something less than a kilobyte?",
            UnknownContent => "Payload is not JSON.",
            Unrecoverable => "Oops! We have encountered an unrecoverable error on our end.",
        })
    }
}

pub type Result<T> = core::result::Result<T, Error>;
