mod error;

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use dashmap::DashMap;
use db::Database;
use tokio::{sync::mpsc, time};
use twilight_model::{
    application::interaction::{ApplicationCommand, Interaction, MessageComponentInteraction},
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{ApplicationMarker, InteractionMarker, UserMarker},
        Id,
    },
};

type Event = (Id<UserMarker>, usize);
type Channel = mpsc::UnboundedSender<Event>;
type QuizRegistry = DashMap<Id<InteractionMarker>, Channel>;

struct Internal {
    /// Container for all pending polls.
    quizzes: QuizRegistry,
    /// Discord API interactions.
    api: twilight_http::Client,
}

impl From<twilight_http::Client> for Internal {
    fn from(api: twilight_http::Client) -> Self {
        Self { api, quizzes: Default::default() }
    }
}

pub struct Lobby {
    /// Internal group of data to clone when detaching a worker.
    inner: Arc<Internal>,
    /// Application ID to match on.
    app: Id<ApplicationMarker>,
}

impl Lobby {
    pub fn new(token: String, app: Id<ApplicationMarker>) -> Self {
        let internal = twilight_http::Client::new(token).into();
        Self { inner: Arc::new(internal), app }
    }

    pub async fn on_interaction(&self, db: &Database, interaction: Interaction) -> InteractionResponse {
        let result = match interaction {
            Interaction::Ping(_) => Ok(InteractionResponse { kind: InteractionResponseType::Pong, data: None }),
            Interaction::ApplicationCommand(comm) => self.on_app_comm(db, *comm).await,
            Interaction::MessageComponent(msg) => self.on_msg_interaction(*msg),
            _ => Err(error::Error::UnsupportedInteraction),
        };

        use alloc::string::ToString;
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

    async fn on_app_comm(&self, db: &Database, comm: ApplicationCommand) -> error::Result<InteractionResponse> {
        match comm.data.name.as_str() {
            "start" => {
                use twilight_model::user::User;
                let User { id, .. } = comm.user.ok_or(error::Error::UnknownUser)?;
                let token = comm.token.into_boxed_str();
                self.on_start_command(db, id, comm.id, token).await
            }
            "help" => Ok(Self::on_help_command()),
            _ => Err(error::Error::UnknownCommandName),
        }
    }

    async fn on_start_command(
        &self,
        db: &Database,
        user: Id<UserMarker>,
        interaction: Id<InteractionMarker>,
        token: Box<str>,
    ) -> error::Result<InteractionResponse> {
        use model::quiz::Quiz;
        let Quiz { question, choices, answer, timeout } =
            db.get_quiz(user).await.map_err(|_| error::Error::UnknownUser)?.ok_or(error::Error::UnknownQuiz)?;
        let (tx, mut rx) = mpsc::unbounded_channel();

        use dashmap::mapref::entry::Entry::Vacant;
        if let Vacant(entry) = self.inner.quizzes.entry(interaction) {
            entry.insert(tx);
        } else {
            return Err(error::Error::Unrecoverable);
        }

        let answer = usize::from(answer);
        let correct = choices.get(answer).ok_or(error::Error::Unrecoverable)?.clone().into_boxed_str();

        let inner = self.inner.clone();
        let app_id = self.app;
        tokio::spawn(async move {
            use alloc::collections::BTreeSet;
            use core::time::Duration;

            // Keep processing new answers
            let mut selections = BTreeSet::new();
            let timer = time::sleep(Duration::from_secs(timeout.into()));
            tokio::pin!(timer);
            loop {
                let (user, choice) = tokio::select! {
                    biased;
                    Some(pair) = rx.recv() => pair,
                    _ = &mut timer => break,
                    else => unreachable!(),
                };
                if choice == answer {
                    selections.insert(user);
                } else {
                    selections.remove(&user);
                }
            }

            // Disable all communication channels
            drop(rx);
            assert!(inner.quizzes.remove(&interaction).is_some());

            // Disable components from original message
            let client = inner.api.interaction(app_id);
            client.update_response(&token).components(Some(&[])).unwrap().exec().await.unwrap();

            // Generate congratulations
            let winners: Vec<_> = selections.into_iter().map(|user| alloc::format!("<@{user}>")).collect();
            let content = if winners.is_empty() {
                alloc::format!("The correct answer is: ||{correct}||. Nobody got it right...")
            } else {
                let congrats = winners.join(" ");
                alloc::format!("The correct answer is: ||{correct}||. Congratulations to {congrats}!")
            };
            drop(winners);

            // Issue follow-up message for winners
            use twilight_model::channel::message::{allowed_mentions::ParseTypes, AllowedMentions};
            client
                .create_followup(&token)
                .content(&content)
                .unwrap()
                .allowed_mentions(Some(&AllowedMentions {
                    parse: alloc::vec![ParseTypes::Users],
                    ..Default::default()
                }))
                .exec()
                .await
                .unwrap();
        });

        use alloc::string::ToString;
        use twilight_model::application::component::{select_menu::SelectMenuOption, ActionRow, Component, SelectMenu};
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
        let comps = alloc::vec![Component::ActionRow(ActionRow {
            components: alloc::vec![Component::SelectMenu(SelectMenu {
                options,
                custom_id: interaction.to_string(),
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
        use twilight_model::channel::embed::{Embed, EmbedField};
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
    fn on_msg_interaction(&self, mut msg: MessageComponentInteraction) -> error::Result<InteractionResponse> {
        use twilight_model::{application::component::ComponentType::SelectMenu, user::User};
        if !matches!(msg.data.component_type, SelectMenu) {
            return Err(error::Error::UnsupportedInteraction);
        }

        let User { id, .. } = msg.member.and_then(|m| m.user).or(msg.user).ok_or(error::Error::UnknownUser)?;

        // Since we know that there can only be one value from this interaction,
        // we simply pop the arguments directly. This allows O(1) deletion.
        let arg = msg.data.values.pop().ok_or(error::Error::Unrecoverable)?;
        let choice = arg.parse().map_err(|_| error::Error::InvalidParams)?;
        drop(arg);

        let quiz_id = msg.data.custom_id.parse().map_err(|_| error::Error::Unrecoverable)?;
        self.inner
            .quizzes
            .get(&quiz_id)
            .ok_or(error::Error::UnknownQuiz)?
            .send((id, choice))
            .map_err(|_| error::Error::Unrecoverable)?;

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
