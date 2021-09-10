use std::{
    env::{var, VarError},
    io::Error as IoError,
    num::NonZeroU64,
};
use tokio::runtime::Builder;

#[derive(Debug)]
enum AppError {
    MissingEnvVars,
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

fn main() -> Result<(), AppError> {
    // Retrieve environment variables
    let bot_token = var("BOT_TOKEN")?;
    let application_id = var("APPLICATION_ID")?
        .parse::<u64>()
        .map_err(|_| AppError::MissingEnvVars)?;
    let guild_id = var("GUILD_ID")?
        .parse::<u64>()
        .ok()
        .and_then(NonZeroU64::new);

    // Launch Tokio async runtime
    let runtime = Builder::new_current_thread().enable_all().build()?;
    Ok(())
}
