mod error;

use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use core::num::{NonZeroI16, NonZeroU64};
use db::Database;
use tokio::sync::mpsc;
use twilight_model::{
    application::interaction::{
        application_command::{CommandData, CommandDataOption, CommandOptionValue},
        message_component::MessageComponentInteractionData,
        Interaction, InteractionData, InteractionType,
    },
    channel::message::{
        component::{ComponentType, SelectMenu, SelectMenuOption},
        embed::{EmbedAuthor, EmbedField},
        Component, Embed, MessageFlags,
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{
        marker::{ApplicationMarker, UserMarker},
        Id,
    },
    user::User,
};

type AppId = Id<ApplicationMarker>;
type UserId = Id<UserMarker>;

struct Event {
    user: UserId,
    choice: u32,
}

type Channel = mpsc::UnboundedSender<Event>;
type Registry = dashmap::DashMap<(UserId, NonZeroI16), Channel>;

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
    pub fn new(db: Database, id: NonZeroU64, token: String) -> Self {
        Self {
            inner: Arc::new(Inner { client: twilight_http::Client::new(token), quizzes: Registry::new() }),
            db,
            id: Id::from(id),
        }
    }

    pub async fn on_message(&self, interaction: Interaction) -> InteractionResponse {
        let result = match interaction.kind {
            InteractionType::Ping => Ok(InteractionResponse { kind: InteractionResponseType::Pong, data: None }),
            InteractionType::ApplicationCommand => self.on_app_command(interaction).await,
            InteractionType::MessageComponent => self.on_msg_component(interaction).await,
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

    async fn on_app_command(&self, interaction: Interaction) -> error::Result<InteractionResponse> {
        let user =
            interaction.member.and_then(|member| member.user).xor(interaction.user).ok_or(error::Error::UnknownUser)?;
        let data = interaction.data.ok_or(error::Error::Fatal)?;
        let InteractionData::ApplicationCommand(data) = data else {
            return Err(error::Error::Fatal);
        };

        let token = interaction.token.into_boxed_str();
        let CommandData { name, options, .. } = *data;

        match name.as_str() {
            "create" => self.on_create_command(user.id, &options).await,
            "list" => self.on_list_command(user).await,
            "add" => self.on_add_choice(user.id, &options).await,
            "remove" => self.on_remove_choice(user.id, &options).await,
            "edit" => self.on_edit_command(user.id, &options).await,
            "start" => self.on_start_command(user.id, &options, token).await,
            "help" => todo!(),
            _ => Err(error::Error::Fatal),
        }
    }

    async fn on_create_command(
        &self,
        uid: Id<UserMarker>,
        options: &[CommandDataOption],
    ) -> error::Result<InteractionResponse> {
        let option = options.first().ok_or(error::Error::InvalidParams)?;
        let CommandDataOption { name, value: CommandOptionValue::String(value) } = option else {
            return Err(error::Error::InvalidParams);
        };

        if name.as_str() != "question" {
            return Err(error::Error::UnknownCommandName);
        }

        let qid = match self.db.init_quiz(uid.into_nonzero(), value.as_str()).await {
            Ok(id) => id,
            Err(db::error::Error::BadInput) => return Err(error::Error::InvalidParams),
            _ => return Err(error::Error::Fatal),
        };

        Ok(InteractionResponse {
            kind: InteractionResponseType::ChannelMessageWithSource,
            data: Some(InteractionResponseData {
                content: Some(alloc::format!("New quiz added: `{qid}`.")),
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
            .map_err(|_| error::Error::Fatal)?
            .map_ok(|db::Quiz { id, raw: db::RawQuiz { question, expiration, choices, .. } }| {
                let fields = choices
                    .into_iter()
                    .zip(1..)
                    .map(|(choice, id)| EmbedField {
                        inline: false,
                        name: alloc::format!("Choice {id}"),
                        value: choice,
                    })
                    .collect();
                Embed {
                    fields,
                    kind: String::from("rich"),
                    color: Some(user.accent_color.unwrap_or(0x236EA5)),
                    title: Some(question),
                    description: Some(alloc::format!("Quiz `{id}` is set to expire in {expiration} seconds.")),
                    author: Some(EmbedAuthor {
                        name: alloc::format!("{}#{}", user.name, user.discriminator()),
                        icon_url: user
                            .avatar
                            .map(|hash| alloc::format!("https://cdn.discordapp.com/avatars/{}/{hash}.webp", user.id)),
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
            .map_err(|_| error::Error::Fatal)
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
        let [
            CommandDataOption { name: qid_arg, value: CommandOptionValue::Integer(qid) },
            CommandDataOption { name: choice_arg, value: CommandOptionValue::String(choice) },
        ] = options else {
            return Err(error::Error::InvalidParams);
        };

        if qid_arg.as_str() != "quiz" || choice_arg.as_str() != "choice" {
            return Err(error::Error::UnknownCommandName);
        }

        let qid = i16::try_from(*qid).map_err(|_| error::Error::UnknownQuiz)?;
        let qid = NonZeroI16::new(qid).ok_or(error::Error::UnknownQuiz)?;
        let Err(err) = self.db.add_choice(uid.into_nonzero(), qid, choice.as_str()).await else {
            return Ok(InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    content: Some(alloc::format!("Successfully added new choice to quiz **[{qid}]**.")),
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..Default::default()
                }),
            });
        };

        use db::error::Error as DbError;
        Err(match err {
            DbError::NotFound => error::Error::UnknownQuiz,
            DbError::BadInput | DbError::TooMany => error::Error::InvalidParams,
            DbError::Fatal => error::Error::Fatal,
        })
    }

    async fn on_remove_choice(&self, uid: UserId, options: &[CommandDataOption]) -> error::Result<InteractionResponse> {
        let [
            CommandDataOption { name: qid_arg, value: CommandOptionValue::Integer(qid) },
            CommandDataOption { name: index_arg, value: CommandOptionValue::Integer(index) },
        ] = options else {
            return Err(error::Error::InvalidParams);
        };

        if qid_arg.as_str() != "quiz" || index_arg.as_str() != "index" {
            return Err(error::Error::UnknownCommandName);
        }

        let qid = i16::try_from(*qid).map_err(|_| error::Error::UnknownQuiz)?;
        let qid = NonZeroI16::new(qid).ok_or(error::Error::UnknownQuiz)?;
        let index = u16::try_from(*index).map_err(|_| error::Error::InvalidParams)?;
        match self.db.remove_choice(uid.into_nonzero(), qid, index).await {
            Ok(choice) => Ok(InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    content: Some(alloc::format!("Successfully removed choice ||{choice}|| from quiz **[{qid}]**. The answer has also been reset.")),
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..Default::default()
                }),
            }),
            Err(db::error::Error::NotFound) => Err(error::Error::UnknownQuiz),
            _ => Err(error::Error::Fatal),
        }
    }

    async fn on_edit_command(&self, uid: UserId, options: &[CommandDataOption]) -> error::Result<InteractionResponse> {
        todo!()
    }

    async fn on_start_command(
        &self,
        uid: UserId,
        options: &[CommandDataOption],
        token: Box<str>,
    ) -> error::Result<InteractionResponse> {
        let option = options.first().ok_or(error::Error::InvalidParams)?;
        let CommandDataOption { name, value: CommandOptionValue::Integer(qid) } = option else {
            return Err(error::Error::InvalidParams);
        };

        if name != "start" {
            return Err(error::Error::UnknownCommandName);
        }

        let qid = i16::try_from(*qid).map_err(|_| error::Error::UnknownQuiz)?;
        let qid = NonZeroI16::new(qid).ok_or(error::Error::UnknownQuiz)?;
        let db::RawQuiz { question, choices, answer, expiration } =
            match self.db.pop_quiz(uid.into_nonzero(), qid).await {
                Ok(quiz) => quiz,
                Err(db::error::Error::NotFound) => return Err(error::Error::UnknownQuiz),
                _ => return Err(error::Error::Fatal),
            };

        let key = (uid, qid);
        let (tx, mut rx) = mpsc::unbounded_channel();
        if self.inner.quizzes.insert(key, tx).is_some() {
            return Err(error::Error::Fatal);
        }

        let app_id = self.id;
        let duration = core::time::Duration::from_secs(expiration.into());
        let inner = self.inner.clone();
        let correct = choices[usize::try_from(answer).unwrap()].clone();
        tokio::spawn(async move {
            let mut users = alloc::collections::BTreeSet::new();
            let mut sleep = core::pin::pin!(tokio::time::sleep(duration));
            loop {
                let Event { user, choice } = tokio::select! {
                    Some(msg) = rx.recv() => msg,
                    _ = &mut sleep => break,
                    else => break,
                };
                if answer == choice {
                    users.insert(user);
                } else {
                    users.remove(&user);
                }
            }

            drop(rx);
            inner.quizzes.remove(&key);

            let mentions: Vec<_> = users.into_iter().map(|user| alloc::format!("<@{user}>")).collect();
            let mentions = mentions.join(" ").into_boxed_str();
            let content = alloc::format!("The correct answer is: ||{correct}||. Congratulations to {mentions}!");
            inner.client.interaction(app_id).create_followup(&token).content(&content).unwrap().await.unwrap();
        });

        use alloc::string::ToString;
        Ok(InteractionResponse {
            kind: InteractionResponseType::DeferredUpdateMessage,
            data: Some(InteractionResponseData {
                content: Some(question),
                components: Some(alloc::vec![Component::SelectMenu(SelectMenu {
                    custom_id: alloc::format!("{uid}:{qid}"),
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
                })]),
                ..Default::default()
            }),
        })
    }

    async fn on_msg_component(&self, interaction: Interaction) -> error::Result<InteractionResponse> {
        let User { id, .. } =
            interaction.member.and_then(|member| member.user).xor(interaction.user).ok_or(error::Error::UnknownUser)?;
        let data = interaction.data.ok_or(error::Error::Fatal)?;
        let InteractionData::MessageComponent(MessageComponentInteractionData {
            component_type: ComponentType::SelectMenu,
            custom_id,
            values,
        }) = data else {
            return Err(error::Error::Fatal);
        };

        let mut iter = custom_id.splitn(2, ':');
        let first = iter.next();
        let second = iter.next();
        if iter.next().is_some() {
            return Err(error::Error::Fatal);
        }

        let (uid, qid) = first.zip(second).ok_or(error::Error::Fatal)?;
        let uid: NonZeroU64 = uid.parse().map_err(|_| error::Error::Fatal)?;
        let qid: NonZeroI16 = qid.parse().map_err(|_| error::Error::Fatal)?;

        let choice = values.into_iter().next().ok_or(error::Error::Fatal)?.parse().map_err(|_| error::Error::Fatal)?;
        self.inner
            .quizzes
            .get(&(Id::from(uid), qid))
            .ok_or(error::Error::UnknownQuiz)?
            .send(Event { user: id, choice })
            .map_err(|_| error::Error::UnknownQuiz)?;

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
