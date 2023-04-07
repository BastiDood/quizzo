pub enum Error {
    /// Quiz does not exist.
    NotFound,
    /// Input cannot be accepted due to data constraints.
    BadInput,
    /// An unexpected and unrecoverable error.
    Fatal,
}

pub type Result<T> = ::core::result::Result<T, Error>;
