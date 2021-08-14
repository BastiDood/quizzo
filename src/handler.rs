use crate::model::Quiz;
use hyper::{
    body::{to_bytes, Bytes},
    client::HttpConnector,
    Client, Uri,
};
use hyper_rustls::HttpsConnector;
use itertools::Itertools;
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
use slab::Slab;
use std::{
    borrow::Cow,
    collections::HashSet,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::{sync::Mutex, time::sleep};

const START_COMMAND_NAME: &str = "start";
const START_COMMAND_ARG: &str = "url";

pub struct Handler {
    http: Client<HttpsConnector<HttpConnector>>,
    guild_id: u64,
    command_id: AtomicU64,
    quizzes: Mutex<Slab<(usize, HashSet<Box<str>>)>>,
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
                let bytes = self
                    .fetch(value)
                    .await
                    .expect("cannot convert body to bytes");
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

                // Register the quiz
                let quiz_id = {
                    let mut quizzes = self.quizzes.lock().await;
                    quizzes.insert((answer, Default::default()))
                };

                // Respond to the user
                let component_options: Vec<_> = choices
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(index, choice)| {
                        json!({
                            "label": choice,
                            "value": index.to_string(),
                        })
                    })
                    .collect();
                let response_options = json!({
                    "type": 4,
                    "data": {
                        "content": question,
                        "components": [
                            {
                                "type": 1,
                                "components": [
                                    {
                                        "type": 3,
                                        "custom_id": quiz_id.to_string(),
                                        "placeholder": "Your Answer",
                                        "options": component_options,
                                    }
                                ],
                            }
                        ],
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

                // Execute the quiz
                sleep(Duration::from_secs(timeout)).await;
                let (_, tally) = {
                    let mut quizzes = self.quizzes.lock().await;
                    quizzes.remove(quiz_id)
                };

                // Count the tally
                #[allow(unstable_name_collisions)]
                let mentions = if tally.is_empty() {
                    Cow::Borrowed("Nobody got the correct answer...")
                } else {
                    let winners: String = tally
                        .iter()
                        .map(|id| Cow::Owned(format!("<@{}>", id)))
                        .intersperse(Cow::Borrowed(" "))
                        .collect();
                    Cow::Owned(format!("Congratulations to {}!", winners))
                };

                // Notify users of quiz result
                let notify_options = json!({
                    "type": 4,
                    "data": {
                        "content": format!("The correct answer is **{}**. {}", choices[answer], mentions),
                        "allowed_mentions": tally,
                    },
                });
                ctx.http
                    .create_interaction_response(
                        interaction.id.0,
                        interaction.token.as_str(),
                        &notify_options,
                    )
                    .await
                    .expect("cannot send response");
            }
            InteractionData::MessageComponent(data) => todo!(),
        }
    }
}

impl Handler {
    async fn fetch(&self, uri: Uri) -> hyper::Result<Bytes> {
        let body = self.http.get(uri).await?.into_body();
        to_bytes(body).await
    }
}
