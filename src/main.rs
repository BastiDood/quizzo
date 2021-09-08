use quizzo::{AppError, Handler};
use std::{env::var, num::NonZeroU64};
use tokio::runtime::Builder;

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
    let runtime = Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(Handler::initialize(&bot_token, application_id, guild_id))
}
