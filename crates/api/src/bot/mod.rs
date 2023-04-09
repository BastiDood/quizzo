mod error;

use alloc::{string::String, vec::Vec};
use core::num::NonZeroI16;
use db::Database;
use tokio::sync::mpsc;
use twilight_model::{
    application::interaction::{
        application_command::{CommandData, CommandDataOption, CommandOptionValue},
        Interaction, InteractionData, InteractionType,
    },
    channel::message::{
        embed::{EmbedAuthor, EmbedField},
        Embed, MessageFlags,
    },
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{marker::UserMarker, Id},
    user::User,
};

type UserId = Id<UserMarker>;

struct Event {
    user: UserId,
    choice: u32,
}

type Channel = mpsc::UnboundedSender<Event>;
type Registry = dashmap::DashMap<NonZeroI16, Channel>;

pub struct Bot {
    client: twilight_http::Client,
    quizzes: Registry,
    db: Database,
}

impl Bot {
    pub fn new(db: Database, token: String) -> Self {
        Self { client: twilight_http::Client::new(token), quizzes: Registry::new(), db }
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
        let CommandData { name, options, .. } = *data;
        match name.as_str() {
            "create" => self.on_create_command(user.id, &options).await,
            "list" => self.on_list_command(user).await,
            "add" => self.on_add_choice(user.id, &options).await,
            "remove" => self.on_remove_choice(user.id, &options).await,
            "edit" => self.on_edit_command(user.id, &options).await,
            "start" => self.on_start_command(user.id, &options).await,
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

    async fn on_add_choice(
        &self,
        uid: Id<UserMarker>,
        options: &[CommandDataOption],
    ) -> error::Result<InteractionResponse> {
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

    async fn on_remove_choice(
        &self,
        uid: Id<UserMarker>,
        options: &[CommandDataOption],
    ) -> error::Result<InteractionResponse> {
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
        let Err(err) = self.db.remove_choice(uid.into_nonzero(), qid, index).await else {
            return Ok(InteractionResponse {
                kind: InteractionResponseType::ChannelMessageWithSource,
                data: Some(InteractionResponseData {
                    content: Some(alloc::format!("Successfully removed choice `{index}` from quiz **[{qid}]**. The answer has also been reset.")),
                    flags: Some(MessageFlags::EPHEMERAL),
                    ..Default::default()
                }),
            });
        };

        use db::error::Error as DbError;
        Err(match err {
            DbError::NotFound => error::Error::UnknownQuiz,
            _ => error::Error::Fatal,
        })
    }

    async fn on_edit_command(
        &self,
        uid: Id<UserMarker>,
        options: &[CommandDataOption],
    ) -> error::Result<InteractionResponse> {
        todo!()
    }

    async fn on_start_command(
        &self,
        uid: Id<UserMarker>,
        options: &[CommandDataOption],
    ) -> error::Result<InteractionResponse> {
        todo!()
    }

    async fn on_msg_component(&self, interaction: Interaction) -> error::Result<InteractionResponse> {
        todo!()
    }
}
