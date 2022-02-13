mod quiz;

use dashmap::DashMap;
use hyper::Client;
use hyper_trust_dns::RustlsHttpsConnector;
use tokio::sync::mpsc;

use twilight_http::client::InteractionClient;
use twilight_model::{
    application::{
        command::{ChoiceCommandOptionData, CommandOption},
        interaction::{ApplicationCommand, MessageComponentInteraction},
    },
    id::{
        marker::{CommandMarker, GuildMarker, InteractionMarker},
        Id,
    },
};

type Key = Id<InteractionMarker>;
type Channel = mpsc::Sender<()>;

pub struct Lobby<'client> {
    /// Container for all pending polls.
    quizzes: DashMap<Key, Channel>,
    /// Discord API interactions.
    api: InteractionClient<'client>,
    /// Arbitrary HTTP fetching of JSON files.
    http: Client<RustlsHttpsConnector>,
    /// Command ID to match on.
    command: Id<CommandMarker>,
}

impl<'c> Lobby<'c> {
    pub fn new(api: InteractionClient<'c>, command: Id<CommandMarker>) -> Self {
        let connector = hyper_trust_dns::new_rustls_native_https_connector();
        let http = Client::builder().http2_only(true).build(connector);
        Self {
            api,
            http,
            command,
            quizzes: DashMap::new(),
        }
    }
}

impl Lobby<'_> {
    /// Registers the quiz creation command.
    pub async fn register<'c>(
        api: InteractionClient<'c>,
        maybe_guild_id: Option<Id<GuildMarker>>,
    ) -> anyhow::Result<Id<CommandMarker>> {
        const CREATE_NAME: &str = "create";
        const CREATE_DESC: &str = "Create a quiz from JSON data.";

        let options = [CommandOption::String(ChoiceCommandOptionData {
            autocomplete: false,
            choices: Vec::new(),
            description: String::from("URL from which to fetch JSON data."),
            required: true,
            name: String::from("url"),
        })];

        let command_fut = if let Some(guild_id) = maybe_guild_id {
            api.create_guild_command(guild_id)
                .chat_input(CREATE_NAME, CREATE_DESC)
                .unwrap()
                .command_options(&options)
                .unwrap()
                .exec()
        } else {
            api.create_global_command()
                .chat_input(CREATE_NAME, CREATE_DESC)
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

    /// Responds to new application commands.
    pub async fn on_app_comm(&self, comm: ApplicationCommand) {
        todo!()
    }

    /// Responds to message component interactions.
    pub async fn on_msg_interaction(&self, comm: MessageComponentInteraction) {
        todo!()
    }
}
