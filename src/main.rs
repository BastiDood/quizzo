mod error;
mod handler;
mod model;

use error::AppError;
use handler::Handler;

use serenity::Client;
use std::env::var;
use tokio::runtime::Builder;

fn main() -> Result<(), AppError> {
    let bot_token = var("BOT_TOKEN")?;
    let guild_id = var("GUILD_ID")?
        .parse::<u64>()
        .map_err(|_| AppError::MissingEnvVars)?;
    let runtime = Builder::new_current_thread().build()?;
    runtime.block_on(async move {
        println!("Connecting to Discord...");
        let mut client = Client::builder(bot_token)
            .event_handler(Handler::from(guild_id))
            .await?;
        println!("Starting client...");
        Ok(client.start().await?)
    })
}
