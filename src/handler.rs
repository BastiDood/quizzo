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
            ApplicationCommandInteractionData, ApplicationCommandInteractionDataOption,
            ApplicationCommandOptionType, ComponentType, Interaction,
            InteractionApplicationCommandCallbackDataFlags, InteractionData, MessageComponent,
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
    quizzes: Mutex<Slab<(usize, HashSet<u64>)>>,
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

        let command = match ctx
            .http
            .create_guild_application_command(self.guild_id, &start_command_opts)
            .await
        {
            Ok(comm) => comm,
            Err(err) => {
                eprintln!("Cannot initialize bot.");
                eprintln!("{}", err);
                return;
            }
        };

        self.command_id.store(command.id.into(), Ordering::Release);
        println!("Bot is ready!");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let result = match interaction.data {
            Some(InteractionData::ApplicationCommand(ApplicationCommandInteractionData {
                id,
                name,
                options,
                ..
            })) if name == START_COMMAND_NAME && id == self.command_id.load(Ordering::Acquire) => {
                self.execute_start_command(
                    &ctx.http,
                    interaction.id.0,
                    interaction.token.as_str(),
                    options.as_slice(),
                )
                .await
            }
            Some(InteractionData::MessageComponent(MessageComponent {
                custom_id,
                component_type: ComponentType::SelectMenu,
                values,
                ..
            })) => {
                let user_id = match interaction
                    .member
                    .map(|member| member.user)
                    .xor(interaction.user)
                {
                    Some(user) => user.id.0,
                    _ => return,
                };

                self.execute_component_response(
                    &ctx.http,
                    interaction.id.0,
                    interaction.token.as_str(),
                    custom_id.as_str(),
                    user_id,
                    values.as_slice(),
                )
                .await
            }
            _ => Err(SlashCommandError::Unrecognized),
        };
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
        } = from_slice::<Quiz>(&bytes)?;

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
        let notify_options = json!({
            "type": 4,
            "data": {
                "content": format!("The correct answer is **{}**. {}", choices[answer], mentions),
                "allowed_mentions": tally,
            },
        });
        ctx.create_interaction_response(id, token, &notify_options)
            .await?;
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
        let quiz_id = custom_id
            .parse()
            .map_err(|_| SlashCommandError::MalformedInput)?;

        let choice = values
            .first()
            .and_then(|val| val.parse::<usize>().ok())
            .ok_or(SlashCommandError::InvalidArgs)?;

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

        let response_options = json!({
            "type": 4,
            "data": {
                "content": "I have received your response.",
                "flags": InteractionApplicationCommandCallbackDataFlags::EPHEMERAL,
            },
        });
        ctx.create_interaction_response(id, token, &response_options)
            .await?;

        Ok(())
    }
}
