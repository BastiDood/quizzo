mod error;

use alloc::{string::String, vec::Vec};
use dashmap::DashMap;
use tokio::sync::mpsc;
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

pub struct Lobby {
    /// Container for all pending polls.
    quizzes: QuizRegistry,
    /// Discord API interactions.
    api: twilight_http::Client,
    /// Application ID to match on.
    app: Id<ApplicationMarker>,
}

impl Lobby {
    pub fn new(token: String, app: Id<ApplicationMarker>) -> Self {
        let api = twilight_http::Client::new(token);
        Self { quizzes: Default::default(), api, app }
    }

    pub fn on_interaction(&self, interaction: Interaction) -> InteractionResponse {
        let result = match interaction {
            Interaction::Ping(_) => Ok(InteractionResponse { kind: InteractionResponseType::Pong, data: None }),
            Interaction::ApplicationCommand(comm) => self.on_app_comm(*comm),
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

    /// Responds to new application commands.
    fn on_app_comm(&self, comm: ApplicationCommand) -> error::Result<InteractionResponse> {
        match comm.data.name.as_str() {
            "start" => self.on_start_command(),
            "help" => Ok(Self::on_help_command()),
            _ => Err(error::Error::UnknownCommandName),
        }
    }

    fn on_start_command(&self) -> error::Result<InteractionResponse> {
        todo!()
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
        let choice = arg.parse().map_err(|_| error::Error::Data)?;
        drop(arg);

        let quiz_id = msg.data.custom_id.parse().map_err(|_| error::Error::Unrecoverable)?;
        self.quizzes
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
