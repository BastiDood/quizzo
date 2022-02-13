mod quiz;

use dashmap::DashMap;
use hyper_trust_dns::RustlsHttpsConnector;
use tokio::sync::mpsc;

use twilight_http::client::InteractionClient;
use twilight_model::{
    application::{
        callback::{CallbackData, InteractionResponse},
        command::{ChoiceCommandOptionData, CommandOption},
        interaction::{ApplicationCommand, Interaction, MessageComponentInteraction},
    },
    channel::message::MessageFlags,
    id::{
        marker::{ApplicationMarker, CommandMarker, GuildMarker, InteractionMarker},
        Id,
    },
};

type Key = Id<InteractionMarker>;
type Channel = mpsc::Sender<()>;

pub struct Lobby {
    /// Container for all pending polls.
    quizzes: DashMap<Key, Channel>,
    /// Discord API interactions.
    api: twilight_http::Client,
    /// Arbitrary HTTP fetching of JSON files.
    http: hyper::Client<RustlsHttpsConnector>,
    /// Application ID to match on.
    app: Id<ApplicationMarker>,
    /// Command ID to match on.
    command: Id<CommandMarker>,
}

impl Lobby {
    const CREATE_NAME: &'static str = "create";
    const CREATE_DESC: &'static str = "Create a quiz from JSON data.";

    /// Registers the quiz creation command.
    async fn register(
        api: InteractionClient<'_>,
        maybe_guild_id: Option<Id<GuildMarker>>,
    ) -> anyhow::Result<Id<CommandMarker>> {
        let options = [CommandOption::String(ChoiceCommandOptionData {
            autocomplete: false,
            choices: Vec::new(),
            description: String::from("URL from which to fetch JSON data."),
            required: true,
            name: String::from("url"),
        })];

        let command_fut = if let Some(guild_id) = maybe_guild_id {
            api.create_guild_command(guild_id)
                .chat_input(Self::CREATE_NAME, Self::CREATE_DESC)
                .unwrap()
                .command_options(&options)
                .unwrap()
                .exec()
        } else {
            api.create_global_command()
                .chat_input(Self::CREATE_NAME, Self::CREATE_DESC)
                .unwrap()
                .command_options(&options)
                .unwrap()
                .exec()
        };

        command_fut
            .await?
            .model()
            .await?
            .id
            .ok_or_else(|| anyhow::Error::msg("absent command ID"))
    }

    pub async fn new(
        token: String,
        app: Id<ApplicationMarker>,
        maybe_guild_id: Option<Id<GuildMarker>>,
    ) -> anyhow::Result<Self> {
        // Initialize Discord API client
        let api = twilight_http::Client::new(token);
        let command = Self::register(api.interaction(app), maybe_guild_id).await?;

        // Initialize HTTP client for fetching JSON
        let connector = hyper_trust_dns::new_rustls_native_https_connector();
        let http = hyper::Client::builder().http2_only(true).build(connector);

        Ok(Self {
            app,
            command,
            api,
            http,
            quizzes: DashMap::new(),
        })
    }

    pub async fn on_interaction(&self, interaction: Interaction) -> InteractionResponse {
        use Interaction::*;
        match interaction {
            Ping(_) => todo!(),
            ApplicationCommand(comm) => self.on_app_comm(*comm).await,
            MessageComponent(msg) => self.on_msg_interaction(*msg).await,
            _ => InteractionResponse::ChannelMessageWithSource(CallbackData {
                content: Some(String::from("Unsupported interaction.")),
                flags: Some(MessageFlags::EPHEMERAL),
                tts: None,
                embeds: None,
                components: None,
                allowed_mentions: None,
            }),
        }
    }

    /// Responds to new application commands.
    pub async fn on_app_comm(&self, comm: ApplicationCommand) -> InteractionResponse {
        if comm.data.id != self.command {
            return InteractionResponse::ChannelMessageWithSource(CallbackData {
                content: Some(String::from("Unknown command ID.")),
                flags: Some(MessageFlags::EPHEMERAL),
                tts: None,
                embeds: None,
                components: None,
                allowed_mentions: None,
            });
        }

        if comm.data.name.as_str() != Self::CREATE_NAME {
            return InteractionResponse::ChannelMessageWithSource(CallbackData {
                content: Some(String::from("Unknown command name.")),
                flags: Some(MessageFlags::EPHEMERAL),
                tts: None,
                embeds: None,
                components: None,
                allowed_mentions: None,
            });
        }

        todo!()
    }

    /// Responds to message component interactions.
    pub async fn on_msg_interaction(&self, msg: MessageComponentInteraction) -> InteractionResponse {
        todo!()
    }
}
