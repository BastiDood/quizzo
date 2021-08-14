use hyper::{client::HttpConnector, Client, Uri};
use hyper_rustls::HttpsConnector;
use serde_json::{json, Value};
use serenity::{
    client::{Context, EventHandler},
    model::{
        interactions::{
            ApplicationCommandOptionType, Interaction,
            InteractionApplicationCommandCallbackDataFlags, InteractionData,
            InteractionResponseType,
        },
        prelude::Ready,
    },
};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

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
    async fn ready(&self, ctx: Context, data: Ready) {
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
                        // TODO: Add response to user
                        let response_options = json!({
                            "type": 4,
                            "data": {
                                "content": "Cannot parse URL.",
                                "flags": InteractionApplicationCommandCallbackDataFlags::EPHEMERAL,
                            },
                        });
                        return;
                    }
                };
            }
            InteractionData::MessageComponent(data) => todo!(),
        }
    }
}

impl Handler {
    async fn fetch(uri: Uri) {}
}
