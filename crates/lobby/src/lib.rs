pub mod service;

mod error;
mod validator;

use dashmap::DashMap;
use error::{Error, Result};
use hyper::{
    body::{self, Buf},
    header::{HeaderValue, ACCEPT, CONTENT_LENGTH, CONTENT_TYPE},
    Body, Request,
};
use hyper_trust_dns::{RustlsHttpsConnector, TrustDnsResolver};
use model::quiz::Quiz;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::{sync::mpsc, time};
use twilight_model::{
    application::{
        component::{select_menu::SelectMenuOption, ActionRow, Component, ComponentType, SelectMenu},
        interaction::{
            application_command::{CommandDataOption, CommandOptionValue},
            ApplicationCommand, Interaction, MessageComponentInteraction,
        },
    },
    channel::{
        embed::{Embed, EmbedField},
        message::{allowed_mentions::ParseTypes, AllowedMentions, MessageFlags},
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{ApplicationMarker, InteractionMarker, UserMarker},
        Id,
    },
};

pub const APPLICATION_JSON: &str = "application/json";

type Event = (Id<UserMarker>, usize);
type Channel = mpsc::UnboundedSender<Event>;
type QuizRegistry = DashMap<Id<InteractionMarker>, Channel>;

#[derive(Clone)]
pub struct Lobby {
    /// Container for all pending polls.
    quizzes: Arc<QuizRegistry>,
    /// Discord API interactions.
    api: Arc<twilight_http::Client>,
    /// Arbitrary HTTP fetching of JSON files.
    http: hyper::Client<RustlsHttpsConnector>,
    /// Application ID to match on.
    app: Id<ApplicationMarker>,
}

impl Lobby {
    const PARAM_NAME: &'static str = "url";

    pub fn new(token: String, app: Id<ApplicationMarker>) -> Self {
        // Initialize Discord API client
        let api = Arc::new(twilight_http::Client::new(token));

        // Initialize HTTP client for fetching JSON
        let connector = TrustDnsResolver::default().into_rustls_native_https_connector();
        let http = hyper::Client::builder().build(connector);

        Self { app, api, http, quizzes: Arc::default() }
    }

    pub async fn on_interaction(&self, interaction: Interaction) -> InteractionResponse {
        let result = match interaction {
            Interaction::Ping(_) => Ok(InteractionResponse {
                kind: twilight_model::http::interaction::InteractionResponseType::Pong,
                data: None,
            }),
            Interaction::ApplicationCommand(comm) => self.on_app_comm(*comm).await,
            Interaction::MessageComponent(msg) => self.on_msg_interaction(*msg).await,
            _ => Err(Error::UnsupportedInteraction),
        };

        let text = match result {
            Ok(res) => return res,
            Err(err) => err.to_string(),
        };

        InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: Some(text),
                flags: Some(MessageFlags::EPHEMERAL),
                tts: None,
                allowed_mentions: None,
                components: None,
                embeds: None,
                attachments: None,
                choices: None,
                custom_id: None,
                title: None,
            }),
        }
    }

    /// Responds to new application commands.
    async fn on_app_comm(&self, comm: ApplicationCommand) -> Result<InteractionResponse> {
        match comm.data.name.as_str() {
            "create" => self.on_create_command(comm).await,
            "help" => Ok(Self::on_help_command()),
            _ => Err(Error::UnknownCommandName),
        }
    }

    async fn on_create_command(&self, mut comm: ApplicationCommand) -> Result<InteractionResponse> {
        // NOTE: We pop off the values to attain O(1) removal time.
        // This does mean that the validation will fail if there are more
        // than one arguments supplied. This should be alright for now since
        // we don't expect the `create` command to accept more than one argument.
        let (name, value) = match comm.data.options.pop() {
            Some(CommandDataOption { name, value: CommandOptionValue::String(value), .. }) => (name, value),
            _ => return Err(Error::InvalidParams),
        };

        if name.as_str() != Self::PARAM_NAME {
            return Err(Error::UnknownParamName);
        }

        drop(name);
        let uri = value.parse()?;
        drop(value);

        if !validator::is_allowed_uri(&uri) {
            return Err(Error::InvalidUri);
        }

        // Construct JSON request
        let mut request = Request::new(Body::empty());
        request.headers_mut().append(ACCEPT, HeaderValue::from_static(APPLICATION_JSON));
        *request.uri_mut() = uri;

        let response = self.http.request(request).await?;
        let headers = response.headers();

        // Verify the length of the data
        let length_str = headers.get(CONTENT_LENGTH).ok_or(Error::FailedFetch)?;
        let length: u16 = length_str.to_str()?.parse()?;
        if length >= 1024 {
            return Err(Error::TooLarge);
        }

        // Verify that the content type is JSON
        let mime = headers.get(CONTENT_TYPE).ok_or(Error::FailedFetch)?.to_str()?;
        if !mime.starts_with(APPLICATION_JSON) {
            return Err(Error::UnknownContent);
        }

        // Finally commit resources to parsing the JSON
        let buf = body::aggregate(response.into_body()).await?.reader();
        let Quiz { question, choices, timeout, answer } = serde_json::from_reader(buf)?;

        if !(1..=25).contains(&choices.len()) {
            return Err(Error::InvalidParams);
        }

        let answer = usize::from(answer);
        let correct = choices.get(answer).ok_or(Error::Data)?.clone().into_boxed_str();

        // Open channel to receiving new answers
        let (tx, mut rx) = mpsc::unbounded_channel();
        self.quizzes.insert(comm.id, tx);

        // Spawn external Tokio task for handling incoming responses
        let api = Arc::clone(&self.api);
        let quizzes = Arc::clone(&self.quizzes);
        let app_id = self.app;
        tokio::spawn(async move {
            // Keep processing new answers
            let mut selections = HashSet::new();
            let timer = time::sleep(Duration::from_secs(timeout.into()));
            tokio::pin!(timer);
            loop {
                let (user, choice) = tokio::select! {
                    biased;
                    Some(pair) = rx.recv() => pair,
                    _ = &mut timer => break,
                    else => anyhow::bail!("unreachable state"),
                };
                if choice == answer {
                    selections.insert(user);
                } else {
                    selections.remove(&user);
                }
            }

            // Disable all communication channels
            drop(rx);
            quizzes.remove(&comm.id);
            drop(quizzes);

            // Disable components from original message
            let client = api.interaction(app_id);
            client.update_response(&comm.token).components(Some(&[]))?.exec().await?;

            // Generate congratulations
            let winners: Vec<_> = selections.into_iter().map(|user| format!("<@{user}>")).collect();
            let content = if winners.is_empty() {
                format!("The correct answer is: ||{correct}||. Nobody got it right...")
            } else {
                let congrats = winners.join(" ");
                format!("The correct answer is: ||{correct}||. Congratulations to {congrats}!")
            };
            drop(winners);

            // Issue follow-up message for winners
            client
                .create_followup(&comm.token)
                .content(&content)?
                .allowed_mentions(Some(&AllowedMentions { parse: vec![ParseTypes::Users], ..Default::default() }))
                .exec()
                .await?;
            anyhow::Ok(())
        });

        let options = choices
            .into_iter()
            .enumerate()
            .map(|(i, label)| SelectMenuOption {
                label,
                description: None,
                emoji: None,
                default: false,
                value: i.to_string(),
            })
            .collect();
        let comps = vec![Component::ActionRow(ActionRow {
            components: vec![Component::SelectMenu(SelectMenu {
                options,
                custom_id: comm.id.to_string(),
                placeholder: Some(String::from("Your Selection")),
                disabled: false,
                min_values: Some(1),
                max_values: Some(1),
            })],
        })];

        Ok(InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: Some(question),
                components: Some(comps),
                flags: None,
                tts: None,
                allowed_mentions: None,
                embeds: None,
                attachments: None,
                choices: None,
                custom_id: None,
                title: None,
            }),
        })
    }

    fn on_help_command() -> InteractionResponse {
        InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: None,
                flags: Some(MessageFlags::EPHEMERAL),
                components: None,
                tts: None,
                allowed_mentions: None,
                embeds: Some(Vec::from([Embed {
                    author: None,
                    color: None,
                    footer: None,
                    image: None,
                    provider: None,
                    thumbnail: None,
                    timestamp: None,
                    url: None,
                    video: None,
                    kind: String::from("rich"),
                    title: Some(String::from("Quizzo Commands")),
                    description: Some(String::from("Available commands for Quizzo.")),
                    fields: Vec::from([
                        EmbedField {
                            name: String::from("`/create url`"),
                            value: String::from(
                                "Start a quiz at the given URL. Only accepts attachment URIs from Discord's CDN.",
                            ),
                            inline: false,
                        },
                        EmbedField {
                            name: String::from("`/help`"),
                            value: String::from("Summon this help menu!"),
                            inline: false,
                        },
                    ]),
                }])),
                attachments: None,
                choices: None,
                custom_id: None,
                title: None,
            }),
        }
    }

    /// Responds to message component interactions.
    async fn on_msg_interaction(&self, mut msg: MessageComponentInteraction) -> Result<InteractionResponse> {
        if !matches!(msg.data.component_type, ComponentType::SelectMenu) {
            return Err(Error::UnsupportedInteraction);
        }

        let user = msg.member.and_then(|m| m.user).or(msg.user).ok_or(Error::UnknownUser)?.id;

        // Since we know that there can only be one value from this interaction,
        // we simply pop the arguments directly. This allows O(1) deletion.
        let arg = msg.data.values.pop().ok_or(Error::Unrecoverable)?;
        let choice = arg.parse().map_err(|_| Error::Data)?;
        drop(arg);

        let quiz_id: Id<InteractionMarker> = msg.data.custom_id.parse().map_err(|_| Error::Unrecoverable)?;
        self.quizzes.get(&quiz_id).ok_or(Error::UnknownQuiz)?.send((user, choice)).map_err(|_| Error::Unrecoverable)?;

        Ok(InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: Some(String::from("We have received your selection.")),
                flags: Some(MessageFlags::EPHEMERAL),
                components: None,
                tts: None,
                allowed_mentions: None,
                embeds: None,
                attachments: None,
                choices: None,
                custom_id: None,
                title: None,
            }),
        })
    }
}
