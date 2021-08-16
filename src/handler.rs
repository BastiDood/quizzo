use crate::{error::SlashCommandError, model::Quiz};
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
    http::Http,
    model::{
        interactions::{
            application_command::{
                ApplicationCommandInteraction, ApplicationCommandInteractionData,
                ApplicationCommandInteractionDataOption,
            },
            message_component::{
                ComponentType, MessageComponentInteraction, MessageComponentInteractionData,
            },
            Interaction,
        },
        prelude::Ready,
    },
};
use slab::Slab;
use std::{
    borrow::Cow,
    collections::HashSet,
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::{sync::Mutex, time::sleep};

const START_COMMAND_NAME: &str = "start";
const START_COMMAND_ARG: &str = "url";

pub struct Handler {
    http: Client<HttpsConnector<HttpConnector>>,
    guild_id: Option<NonZeroU64>,
    command_id: AtomicU64,
    quizzes: Mutex<Slab<(usize, HashSet<u64>)>>,
}

impl From<Option<NonZeroU64>> for Handler {
    fn from(guild_id: Option<NonZeroU64>) -> Self {
        let connector = HttpsConnector::with_webpki_roots();
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
                    "type": 3,
                    "name": START_COMMAND_ARG,
                    "description": "The URL to which the JSON quiz is found.",
                    "required": true,
                }
            ],
        });

        let maybe_command = if let Some(guild_id) = self.guild_id.map(NonZeroU64::get) {
            ctx.http
                .create_guild_application_command(guild_id, &start_command_opts)
                .await
        } else {
            ctx.http
                .create_global_application_command(&start_command_opts)
                .await
        };

        let command = maybe_command.expect("cannot initialize guild command");
        self.command_id.store(command.id.0, Ordering::Release);
        println!("Bot is ready!");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let result = match &interaction {
            Interaction::ApplicationCommand(ApplicationCommandInteraction {
                id,
                token,
                data:
                    ApplicationCommandInteractionData {
                        id: command_id,
                        name,
                        options,
                        ..
                    },
                ..
            }) if name == START_COMMAND_NAME
                && command_id.0 == self.command_id.load(Ordering::Acquire) =>
            {
                self.execute_start_command(&ctx.http, id.0, token.as_str(), options.as_slice())
                    .await
            }
            Interaction::MessageComponent(MessageComponentInteraction {
                id,
                user,
                token,
                data:
                    MessageComponentInteractionData {
                        custom_id,
                        component_type: ComponentType::SelectMenu,
                        values,
                        ..
                    },
                ..
            }) => {
                self.execute_component_response(
                    &ctx.http,
                    id.0,
                    token.as_str(),
                    custom_id.as_str(),
                    user.id.0,
                    values.as_slice(),
                )
                .await
            }
            _ => Err(SlashCommandError::Unrecognized),
        };

        let message = match result {
            Ok(_) => return,
            Err(SlashCommandError::InvalidArgs) => "Invalid arguments.",
            Err(SlashCommandError::MalformedInput) => "Malformed input data.",
            Err(SlashCommandError::FailedFetch) => "Cannot fetch JSON.",
            Err(SlashCommandError::Fatal) => "Fatal error encountered.",
            Err(SlashCommandError::Unrecognized) => "Unrecognized command.",
        };

        let response_options = json!({
            "type": 4,
            "data": {
                "content": message,
                "flags": 64,
            },
        });
        ctx.http
            .create_interaction_response(interaction.id().0, interaction.token(), &response_options)
            .await
            .expect("cannot send interaction response");
    }
}

impl Handler {
    async fn fetch(&self, uri: Uri) -> hyper::Result<Bytes> {
        let body = self.http.get(uri).await?.into_body();
        to_bytes(body).await
    }

    async fn execute_start_command(
        &self,
        ctx: &Http,
        id: u64,
        token: &str,
        options: &[ApplicationCommandInteractionDataOption],
    ) -> Result<(), SlashCommandError> {
        // Check if correct arguments are given
        let argument = match options {
            [arg] if arg.name == START_COMMAND_ARG => arg,
            _ => return Err(SlashCommandError::InvalidArgs),
        };

        // Try to parse the URL
        let value = argument
            .value
            .as_ref()
            .and_then(Value::as_str)
            .and_then(|val| val.parse::<Uri>().ok())
            .ok_or(SlashCommandError::MalformedInput)?;

        // Fetch the JSON quiz
        let bytes = self.fetch(value).await?;
        let Quiz {
            question,
            answer,
            choices,
            timeout,
        } = from_slice(&bytes)?;

        // Validate the quiz
        if answer >= choices.len() || timeout < 15 || timeout > 30 {
            return Err(SlashCommandError::MalformedInput);
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
        ctx.create_interaction_response(id, token, &response_options)
            .await?;

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
        let message = format!(
            "The correct answer is **{}**. {}",
            choices[answer], mentions
        );
        let notify_options = json!({
            "content": message.as_str(),
            "allowed_mentions": { "users": tally },
        });
        ctx.create_followup_message(token, &notify_options).await?;
        Ok(())
    }

    async fn execute_component_response(
        &self,
        ctx: &Http,
        id: u64,
        token: &str,
        custom_id: &str,
        user_id: u64,
        values: &[String],
    ) -> Result<(), SlashCommandError> {
        // Verify arguments
        let quiz_id = custom_id
            .parse()
            .map_err(|_| SlashCommandError::InvalidArgs)?;
        let choice = values
            .first()
            .and_then(|val| val.parse::<usize>().ok())
            .ok_or(SlashCommandError::InvalidArgs)?;

        // Register user's answer
        {
            let mut quizzes = self.quizzes.lock().await;
            let &mut (answer, ref mut tally) = quizzes
                .get_mut(quiz_id)
                .ok_or(SlashCommandError::InvalidArgs)?;
            if choice == answer {
                tally.insert(user_id);
            } else {
                tally.remove(&user_id);
            }
        }

        // Acknowledge response
        let response_options = json!({
            "type": 4,
            "data": {
                "content": "I have received your response.",
                "flags": 1 << 6,
            },
        });
        ctx.create_interaction_response(id, token, &response_options)
            .await?;

        Ok(())
    }
}
