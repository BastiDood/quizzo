mod error;

use alloc::string::String;
use core::num::NonZeroI16;
use db::Database;
use tokio::sync::mpsc;
use twilight_model::{
    application::interaction::{
        application_command::{CommandData, CommandDataOption, CommandOptionValue},
        Interaction, InteractionData, InteractionType,
    },
    channel::message::MessageFlags,
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{marker::UserMarker, Id},
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

    pub fn on_message(&self, interaction: Interaction) -> InteractionResponse {
        let result = match interaction.kind {
            InteractionType::Ping => Ok(InteractionResponse { kind: InteractionResponseType::Pong, data: None }),
            InteractionType::ApplicationCommand => self.on_app_command(interaction),
            InteractionType::MessageComponent => self.on_msg_component(interaction),
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
        let InteractionData::ApplicationCommand(CommandData { name, options, .. }) = data else {
            return Err(error::Error::Fatal);
        };

        match name.as_str() {
            "create" => self.on_create_command(user.id, &options).await,
            "edit" => todo!(),
            "start" => todo!(),
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

    fn on_msg_component(&self, interaction: Interaction) -> error::Result<InteractionResponse> {
        todo!()
    }
}
