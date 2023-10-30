mod error;

use core::num::{NonZeroI16, NonZeroU64};
use db::Database;
use std::sync::Arc;
use tokio::sync::mpsc;
use twilight_model::{
    application::interaction::{
        application_command::{CommandData, CommandDataOption, CommandOptionValue},
        message_component::MessageComponentInteractionData,
        Interaction, InteractionData, InteractionType,
    },
    channel::message::{
        component::{ActionRow, ComponentType, SelectMenu, SelectMenuOption},
        embed::{EmbedAuthor, EmbedField},
        AllowedMentions, Component, Embed, MentionType, MessageFlags,
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{ApplicationMarker, InteractionMarker, UserMarker},
        Id,
    },
    user::User,
};

type AppId = Id<ApplicationMarker>;
type UserId = Id<UserMarker>;
type InteractionId = Id<InteractionMarker>;

struct Event {
    user: UserId,
    choice: u32,
}

type Channel = mpsc::UnboundedSender<Event>;
type Registry = dashmap::DashMap<InteractionId, Channel>;

struct Inner {
    client: twilight_http::Client,
    quizzes: Registry,
}

pub struct Bot {
    inner: Arc<Inner>,
    db: Database,
    id: AppId,
}

impl Bot {
    const BRAND_COLOR: u32 = 0x236EA5;

    pub fn new(db: Database, id: NonZeroU64, token: String) -> Self {
        Self {
            inner: Arc::new(Inner { client: twilight_http::Client::new(token), quizzes: Registry::new() }),
            db,
            id: Id::from(id),
        }
    }

    pub async fn on_message(&self, interaction: Interaction) -> InteractionResponse {
        let result = match interaction.kind {
            InteractionType::Ping => return InteractionResponse { kind: InteractionResponseType::Pong, data: None },
            InteractionType::ApplicationCommand => self.on_app_command(interaction).await,
            InteractionType::MessageComponent => self.on_msg_component(interaction).await,
            _ => Err(error::Error::Schema),
        };
        result.unwrap_or_else(|err| {
            log::error!("Interaction failed with `{err:?}`");
            InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    content: Some(err.to_string()),
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
        })
    }

    async fn on_app_command(&self, interaction: Interaction) -> error::Result<InteractionResponse> {
        let user =
            interaction.member.and_then(|member| member.user).xor(interaction.user).ok_or(error::Error::Schema)?;
        let data = interaction.data.ok_or(error::Error::Schema)?;
        let InteractionData::ApplicationCommand(data) = data else {
            return Err(error::Error::Schema);
        };
        log::info!("{data:?}");

        let iid = interaction.id;
        let token = interaction.token.into_boxed_str();
        let CommandData { name, options, .. } = *data;

        match name.as_str() {
            "create" => self.on_create_command(user.id, &options).await,
            "list" => self.on_list_command(user).await,
            "add" => self.on_add_choice(user.id, &options).await,
            "remove" => self.on_remove_choice(user.id, &options).await,
            "edit" => self.on_edit_command(user.id, &options).await,
            "start" => self.on_start_command(user.id, &options, iid, token).await,
            "help" => Ok(InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    embeds: Some(vec![Embed {
                        color: Some(Self::BRAND_COLOR),
                        title: Some(String::from("Quizzo!")),
                        description: Some(String::from("A list of commands for Quizzo.")),
                        fields: vec![
                            EmbedField {
                                inline: false,
                                name: String::from("`/help`"),
                                value: String::from("Summon the help page."),
                            },
                            EmbedField {
                                inline: false,
                                name: String::from("`/list`"),
                                value: String::from("Lists down your currently active (but not started) quizzes."),
                            },
                            EmbedField {
                                inline: false,
                                name: String::from("`/create <question>`"),
                                value: String::from("Creates a new quiz. Returns the generated quiz ID."),
                            },
                            EmbedField {
                                inline: false,
                                name: String::from("`/add <qid> <choice>`"),
                                value: String::from("Adds a new `<choice>` for quiz `<qid>`."),
                            },
                            EmbedField {
                                inline: false,
                                name: String::from("`/remove <qid> <index>`"),
                                value: String::from("Removes an existing choice by its `<index>` from quiz `<qid>`."),
                            },
                            EmbedField {
                                inline: false,
                                name: String::from("`/edit question <qid> <question>`"),
                                value: String::from("Sets a new question for quiz `<qid>`."),
                            },
                            EmbedField {
                                inline: false,
                                name: String::from("`/edit expiration <qid> <expiration>`"),
                                value: String::from("Sets a new expiration time for quiz `<qid>`."),
                            },
                            EmbedField {
                                inline: false,
                                name: String::from("`/edit answer <qid> <answer>`"),
                                value: String::from("Sets the correct answer for quiz `<qid>`. Expects a zero-indexed"),
                            },
                            EmbedField {
                                inline: false,
                                name: String::from("`/start <qid>`"),
                                value: String::from("Starts quiz `<qid>` in the current channel. The quiz is then removed from the list."),
                            },
                        ],
                        kind: String::from("rich"),
                        author: None,
                        footer: None,
                        image: None,
                        provider: None,
                        thumbnail: None,
                        timestamp: None,
                        url: None,
                        video: None,
                    }]),
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..Default::default()
                }),
            }),
            "about" => Ok(InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    embeds: Some(vec![Embed {
                        color: Some(Self::BRAND_COLOR),
                        title: Some("About Quizzo!".into()),
                        description: Some("Quizzo is an [open-source](https://github.com/BastiDood/quizzo) Discord bot written in [Rust](https://www.rust-lang.org/) for making simple, timed, multiple-choice quizzes.".into()),
                        fields: Vec::new(),
                        kind: "rich".into(),
                        author: Some(EmbedAuthor {
                            icon_url: Some("https://cdn.discordapp.com/avatars/374495340902088704/aa236a66d815d3d204b28806e6305064.png".into()),
                            name: "Basti Ortiz (@bastidood)".into(),
                            url: Some("https://bastidood.github.io/".into()),
                            proxy_icon_url: None,
                        }),
                        footer: None,
                        image: None,
                        provider: None,
                        thumbnail: None,
                        timestamp: None,
                        url: Some("https://github.com/BastiDood/quizzo".into()),
                        video: None,
                    }]),
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..Default::default()
                }),
            }),
            _ => Err(error::Error::Schema),
        }
    }

    async fn on_create_command(
        &self,
        uid: Id<UserMarker>,
        options: &[CommandDataOption],
    ) -> error::Result<InteractionResponse> {
        let option = options.first().ok_or(error::Error::Schema)?;
        let CommandDataOption { name, value: CommandOptionValue::String(value) } = option else {
            return Err(error::Error::Schema);
        };

        if name.as_str() != "question" {
            return Err(error::Error::Schema);
        }

        let qid = match self.db.init_quiz(uid.into_nonzero(), value.as_str()).await {
            Ok(id) => id,
            Err(db::error::Error::BadInput) => return Err(error::Error::BadInput),
            _ => return Err(error::Error::Database),
        };

        Ok(InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: Some(format!("New quiz added: `{qid}`.")),
                flags: Some(MessageFlags::EPHEMERAL),
                ..Default::default()
            }),
        })
    }

    async fn on_list_command(&self, user: User) -> error::Result<InteractionResponse> {
        use db::TryStreamExt;
        let embeds: Vec<_> = self
            .db
            .get_quizzes_by_user(user.id.into_nonzero())
            .await
            .map_err(|_| error::Error::Database)?
            .map_ok(|db::Quiz { id, raw: db::RawQuiz { question, expiration, choices, answer } }| {
                let iter = choices.into_iter().zip(0..);
                let fields = if let Some(answer) = answer {
                    iter.map(|(choice, id)| EmbedField {
                        inline: false,
                        name: if id == answer { format!(":white_check_mark: {id}") } else { format!(":x: {id}") },
                        value: choice,
                    })
                    .collect()
                } else {
                    iter.map(|(choice, id)| EmbedField {
                        inline: false,
                        name: format!(":white_check_mark: {id}"),
                        value: choice,
                    })
                    .collect()
                };
                Embed {
                    fields,
                    kind: String::from("rich"),
                    color: Some(user.accent_color.unwrap_or(Self::BRAND_COLOR)),
                    title: Some(question),
                    description: Some(format!("Quiz `{id}` is set to expire in {expiration} seconds.")),
                    author: Some(EmbedAuthor {
                        name: format!("{}#{}", user.name, user.discriminator()),
                        icon_url: user
                            .avatar
                            .map(|hash| format!("https://cdn.discordapp.com/avatars/{}/{hash}.webp", user.id)),
                        proxy_icon_url: None,
                        url: None,
                    }),
                    footer: None,
                    image: None,
                    provider: None,
                    thumbnail: None,
                    timestamp: None,
                    url: None,
                    video: None,
                }
            })
            .map_err(|_| error::Error::Database)
            .try_collect()
            .await?;
        Ok(InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(if embeds.is_empty() {
                InteractionResponseData {
                    content: Some(String::from("You currently have no quizzes registered.")),
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..Default::default()
                }
            } else {
                InteractionResponseData {
                    embeds: Some(embeds),
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..Default::default()
                }
            }),
        })
    }

    async fn on_add_choice(&self, uid: UserId, options: &[CommandDataOption]) -> error::Result<InteractionResponse> {
        let [CommandDataOption { name: qid_arg, value: CommandOptionValue::Integer(qid) }, CommandDataOption { name: choice_arg, value: CommandOptionValue::String(choice) }] =
            options
        else {
            return Err(error::Error::Schema);
        };

        if qid_arg.as_str() != "quiz" || choice_arg.as_str() != "choice" {
            return Err(error::Error::Schema);
        }

        let qid = i16::try_from(*qid).map_err(|_| error::Error::Schema)?;
        let qid = NonZeroI16::new(qid).ok_or(error::Error::Schema)?;
        let Err(err) = self.db.add_choice(uid.into_nonzero(), qid, choice.as_str()).await else {
            return Ok(InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    content: Some(format!("Successfully added new choice to quiz **[{qid}]**.")),
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..Default::default()
                }),
            });
        };

        use db::error::Error as DbError;
        Err(match err {
            DbError::NotFound => error::Error::NotFound,
            DbError::BadInput | DbError::TooMany => error::Error::BadInput,
            DbError::Fatal => error::Error::Database,
        })
    }

    async fn on_remove_choice(&self, uid: UserId, options: &[CommandDataOption]) -> error::Result<InteractionResponse> {
        let [CommandDataOption { name: qid_arg, value: CommandOptionValue::Integer(qid) }, CommandDataOption { name: index_arg, value: CommandOptionValue::Integer(index) }] =
            options
        else {
            return Err(error::Error::Schema);
        };

        if qid_arg.as_str() != "quiz" || index_arg.as_str() != "index" {
            return Err(error::Error::Schema);
        }

        let qid = i16::try_from(*qid).map_err(|_| error::Error::Schema)?;
        let qid = NonZeroI16::new(qid).ok_or(error::Error::Schema)?;
        let index = u32::try_from(*index).map_err(|_| error::Error::Schema)?;
        match self.db.remove_choice(uid.into_nonzero(), qid, index).await {
            Ok(choice) => Ok(InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    content: Some(format!("Successfully removed choice ||{choice}|| from quiz **[{qid}]**. The answer has also been reset.")),
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..Default::default()
                }),
            }),
            Err(db::error::Error::NotFound) => Err(error::Error::NotFound),
            _ => Err(error::Error::Database),
        }
    }

    async fn on_edit_command(&self, uid: UserId, options: &[CommandDataOption]) -> error::Result<InteractionResponse> {
        let data = options.first().ok_or(error::Error::Schema)?;
        let CommandDataOption { name, value: CommandOptionValue::SubCommand(args) } = data else {
            return Err(error::Error::Schema);
        };

        let [CommandDataOption { name: qid_name, value: CommandOptionValue::Integer(qid) }, CommandDataOption { name: arg_name, value: arg }] =
            args.as_slice()
        else {
            return Err(error::Error::Schema);
        };

        if qid_name.as_str() != "quiz" || name.as_str() != arg_name.as_str() {
            return Err(error::Error::Schema);
        }

        let uid = uid.into_nonzero();
        let qid = i16::try_from(*qid).map_err(|_| error::Error::Schema)?;
        let qid = NonZeroI16::new(qid).ok_or(error::Error::Schema)?;

        let result = match (arg_name.as_str(), arg) {
            ("question", CommandOptionValue::String(question)) => {
                let q = question.as_str();
                self.db.set_question(uid, qid, q).await
            }
            ("answer", CommandOptionValue::Integer(index)) => {
                let idx = u16::try_from(*index).map_err(|_| error::Error::Schema)?;
                self.db.set_answer(uid, qid, idx).await
            }
            ("expiration", CommandOptionValue::Integer(expiration)) => {
                let exp = u16::try_from(*expiration).map_err(|_| error::Error::Schema)?;
                self.db.set_expiration(uid, qid, exp).await
            }
            _ => return Err(error::Error::Schema),
        };

        let Err(err) = result else {
            return Ok(InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    content: Some(format!("The {arg_name} property has been edited.")),
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..Default::default()
                }),
            });
        };

        use db::error::Error as DbError;
        Err(match err {
            DbError::NotFound => error::Error::NotFound,
            DbError::BadInput => error::Error::BadInput,
            _ => error::Error::Database,
        })
    }

    async fn on_start_command(
        &self,
        uid: UserId,
        options: &[CommandDataOption],
        iid: InteractionId,
        token: Box<str>,
    ) -> error::Result<InteractionResponse> {
        let option = options.first().ok_or(error::Error::Schema)?;
        let CommandDataOption { name, value: CommandOptionValue::Integer(qid) } = option else {
            return Err(error::Error::Schema);
        };

        if name != "quiz" {
            return Err(error::Error::Schema);
        }

        let qid = i16::try_from(*qid).map_err(|_| error::Error::Schema)?;
        let qid = NonZeroI16::new(qid).ok_or(error::Error::Schema)?;
        let db::RawQuiz { question, choices, answer, expiration } =
            match self.db.pop_quiz(uid.into_nonzero(), qid).await {
                Ok(quiz) => quiz,
                Err(db::error::Error::NotFound) => return Err(error::Error::NotFound),
                _ => return Err(error::Error::Database),
            };
        let Some(answer) = answer else {
            return Err(error::Error::BadInput);
        };

        use std::time::SystemTime;
        let expiration = u64::try_from(expiration).map_err(|_| error::Error::Database)?;
        let duration = core::time::Duration::from_secs(expiration);
        let expires_at = SystemTime::now()
            .checked_add(duration)
            .ok_or(error::Error::Fatal)?
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| error::Error::Fatal)?
            .as_secs();

        let (tx, mut rx) = mpsc::unbounded_channel();
        if self.inner.quizzes.insert(iid, tx).is_some() {
            return Err(error::Error::Fatal);
        }

        let app_id = self.id;
        let inner = self.inner.clone();
        let correct = choices[usize::try_from(answer).unwrap()].clone();
        tokio::spawn(async move {
            let mut users = std::collections::BTreeSet::new();
            let mut sleep = core::pin::pin!(tokio::time::sleep(duration));
            loop {
                let Event { user, choice } = tokio::select! {
                    Some(msg) = rx.recv() => msg,
                    _ = &mut sleep => break,
                    else => break,
                };
                if i64::from(answer) == i64::from(choice) {
                    users.insert(user);
                } else {
                    users.remove(&user);
                }
            }

            drop(rx);
            inner.quizzes.remove(&iid);

            let winners: Vec<_> = users.into_iter().map(|user| format!("<@{user}>")).collect();
            let content = if winners.is_empty() {
                format!("The correct answer is: ||{correct}||. Nobody got it right...")
            } else {
                let mentions = winners.join(" ").into_boxed_str();
                format!("The correct answer is: ||{correct}||. Congratulations to {mentions}!")
            };
            inner
                .client
                .interaction(app_id)
                .create_followup(&token)
                .allowed_mentions(Some(&AllowedMentions { parse: vec![MentionType::Users], ..Default::default() }))
                .content(&content)
                .unwrap()
                .await
                .unwrap();
        });

        Ok(InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: Some(format!("**[Expires <t:{expires_at}:R>]:** {question}")),
                components: Some(vec![Component::ActionRow(ActionRow {
                    components: vec![Component::SelectMenu(SelectMenu {
                        custom_id: iid.to_string(),
                        min_values: Some(1),
                        max_values: Some(1),
                        disabled: false,
                        placeholder: Some(String::from("Your Answer")),
                        options: choices
                            .into_iter()
                            .enumerate()
                            .map(|(id, choice)| SelectMenuOption {
                                default: false,
                                description: None,
                                emoji: None,
                                label: choice,
                                value: id.to_string(),
                            })
                            .collect(),
                    })],
                })]),
                ..Default::default()
            }),
        })
    }

    async fn on_msg_component(&self, interaction: Interaction) -> error::Result<InteractionResponse> {
        let User { id, .. } =
            interaction.member.and_then(|member| member.user).xor(interaction.user).ok_or(error::Error::Schema)?;
        let data = interaction.data.ok_or(error::Error::Schema)?;
        log::info!("{data:?}");

        let InteractionData::MessageComponent(MessageComponentInteractionData {
            component_type: ComponentType::SelectMenu,
            custom_id,
            values,
        }) = data
        else {
            return Err(error::Error::Schema);
        };
        let choice =
            values.into_iter().next().ok_or(error::Error::Schema)?.parse().map_err(|_| error::Error::Schema)?;
        let iid = custom_id.parse().map_err(|_| error::Error::Schema)?;

        self.inner
            .quizzes
            .get(&iid)
            .ok_or(error::Error::NotFound)?
            .send(Event { user: id, choice })
            .map_err(|_| error::Error::NotFound)?;

        Ok(InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: Some(String::from("Your answer has been successfully recorded.")),
                flags: Some(MessageFlags::EPHEMERAL),
                ..Default::default()
            }),
        })
    }
}
