use hyper::Error as HyperError;
use std::{env::VarError, io::Error as IoError};

#[derive(Debug)]
pub enum AppError {
    MissingEnvVars,
    MalformedEnvVars,
    Hyper(HyperError),
    Io(IoError),
}

impl From<VarError> for AppError {
    fn from(_: VarError) -> Self {
        Self::MissingEnvVars
    }
}

impl From<IoError> for AppError {
    fn from(err: IoError) -> Self {
        Self::Io(err)
    }
}

impl From<HyperError> for AppError {
    fn from(err: HyperError) -> Self {
        Self::Hyper(err)
    }
}
