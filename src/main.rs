use quizzo::{AppError, Handler};
use serenity::Client;
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
    let runtime = Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(async move {
        println!("Connecting to Discord...");
        let mut client = Client::builder(bot_token)
            .application_id(application_id)
            .event_handler(Handler::from(guild_id))
            .await?;
        println!("Starting client...");
        Ok(client.start().await?)
    })
}
