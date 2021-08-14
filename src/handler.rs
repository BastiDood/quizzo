use crate::model::Quiz;
use hyper::{body::to_bytes, client::HttpConnector, Client, Uri};
use hyper_rustls::HttpsConnector;
use serde_json::{from_slice, json, Value};
use serenity::{
    client::{Context, EventHandler},
    model::{
        interactions::{
            ApplicationCommandOptionType, Interaction,
            InteractionApplicationCommandCallbackDataFlags, InteractionData,
        },
        prelude::Ready,
    },
};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::time::sleep;

const START_COMMAND_NAME: &str = "start";
const START_COMMAND_ARG: &str = "url";

pub struct Handler {
    http: Client<HttpsConnector<HttpConnector>>,
    guild_id: u64,
    command_id: AtomicU64,
    quizzes: HashMap<Box<str>, u8>,
}

impl From<u64> for Handler {
    fn from(guild_id: u64) -> Self {
        let connector = HttpsConnector::with_native_roots();
        let mut client = Client::builder();
        client.http2_only(true);
        Self {
            guild_id,
            http: client.build(connector),
            command_id: Default::default(),
            quizzes: Default::default(),
        }
    }
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        println!("Registering commands...");
        let start_command_opts = json!({
            "name": START_COMMAND_NAME,
            "description": "Start a new quiz.",
            "options": [
                {
                    "type": ApplicationCommandOptionType::String,
                    "name": START_COMMAND_ARG,
                    "description": "The URL to which the JSON quiz is found.",
                    "required": true,
                }
            ],
        });
        let command = ctx
            .http
            .create_guild_application_command(self.guild_id, &start_command_opts)
            .await
            .expect("cannot initialize slash command");
        self.command_id.store(command.id.into(), Ordering::Release);
        println!("Bot is ready!");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        // Check if the user exists
        let user = match interaction
            .member
            .as_ref()
            .map(|member| &member.user)
            .xor(interaction.user.as_ref())
        {
            Some(pair) => pair,
            _ => return,
        };

        // Check if there exists some interaction data
        let data = match interaction.data.as_ref() {
            Some(data) => data,
            _ => return,
        };

        // Respond to the user
        match data {
            InteractionData::ApplicationCommand(data) => {
                // Check if the command is indeed correct
                let command_id = self.command_id.load(Ordering::Acquire);
                if data.id != command_id || data.name != START_COMMAND_NAME {
                    return;
                }

                // Check if correct arguments are given
                let argument = match data.options.first() {
                    Some(arg) if arg.name == START_COMMAND_ARG => arg,
                    _ => return,
                };

                // Try to parse the URL
                let value = match argument
                    .value
                    .as_ref()
                    .and_then(Value::as_str)
                    .and_then(|val| val.parse::<Uri>().ok())
                {
                    Some(val) => val,
                    _ => {
                        let response_options = json!({
                            "type": 4,
                            "data": {
                                "content": "Cannot parse URL.",
                                "flags": InteractionApplicationCommandCallbackDataFlags::EPHEMERAL,
                            },
                        });
                        ctx.http
                            .create_interaction_response(
                                interaction.id.0,
                                interaction.token.as_str(),
                                &response_options,
                            )
                            .await
                            .expect("cannot send response");
                        return;
                    }
                };

                // Fetch the JSON quiz
                let body = self
                    .http
                    .get(value)
                    .await
                    .expect("cannot get response body")
                    .into_body();
                let bytes = to_bytes(body).await.expect("cannot convert body to bytes");
                let Quiz {
                    question,
                    answer,
                    choices,
                    timeout,
                } = from_slice::<Quiz>(&bytes).expect("failed to deserialize quiz");

                // Validate the quiz
                if answer >= choices.len() || timeout < 15 || timeout > 30 {
                    // TODO: Send response to user
                    return;
                }

                // Execute the quiz
                sleep(Duration::from_secs(timeout)).await;
            }
            InteractionData::MessageComponent(data) => todo!(),
        }
    }
}

impl Handler {
    async fn fetch(uri: Uri) {}
}
