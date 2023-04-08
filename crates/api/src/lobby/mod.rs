mod error;

use alloc::string::String;
use db::{Database, Uuid};
use tokio::sync::mpsc;
use twilight_model::{
    application::interaction::{Interaction, InteractionType},
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
type Registry = dashmap::DashMap<Uuid, Channel>;

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
            InteractionType::ApplicationCommand => todo!(),
            InteractionType::MessageComponent => todo!(),
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
}
