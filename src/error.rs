use serenity::Error as SerenityError;
use std::{env::VarError, io::Error as IoError};

#[derive(Debug)]
pub enum AppError {
    MissingEnvVars,
    Serenity(SerenityError),
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

impl From<SerenityError> for AppError {
    fn from(err: SerenityError) -> Self {
        Self::Serenity(err)
    }
}
