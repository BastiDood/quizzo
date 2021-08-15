use hyper::Error as HyperError;
use serde_json::error::{Category, Error as JsonError};
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

#[derive(Debug)]
pub enum SlashCommandError {
    Unrecognized,
    InvalidArgs,
    MalformedInput,
    FailedFetch,
    Fatal,
}

impl From<HyperError> for SlashCommandError {
    fn from(_: HyperError) -> Self {
        Self::FailedFetch
    }
}

impl From<JsonError> for SlashCommandError {
    fn from(err: JsonError) -> Self {
        match err.classify() {
            Category::Io => Self::FailedFetch,
            _ => Self::MalformedInput,
        }
    }
}

impl From<SerenityError> for SlashCommandError {
    fn from(_: SerenityError) -> Self {
        Self::Fatal
    }
}
