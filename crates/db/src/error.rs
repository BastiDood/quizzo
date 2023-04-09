#[derive(Debug)]
pub enum Error {
    /// Quiz does not exist.
    NotFound,
    /// Input cannot be accepted due to data constraints.
    BadInput,
    /// Attempted to insert too many choices.
    TooMany,
    /// An unexpected and unrecoverable error.
    Fatal,
}

pub type Result<T> = ::core::result::Result<T, Error>;
