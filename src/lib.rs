mod error;
mod quiz;

use error::{Error, Result};
use quiz::Quiz;

use dashmap::DashMap;
use hyper::body::{self, Buf};
use hyper_trust_dns::RustlsHttpsConnector;
use tokio::sync::mpsc;

use twilight_http::client::InteractionClient;
use twilight_model::{
    application::{
        callback::{CallbackData, InteractionResponse},
        command::{ChoiceCommandOptionData, CommandOption},
        component::{select_menu::SelectMenuOption, ActionRow, Component, SelectMenu},
        interaction::{
            application_command::{CommandDataOption, CommandOptionValue},
            ApplicationCommand, Interaction, MessageComponentInteraction,
        },
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
    const PARAM_NAME: &'static str = "url";
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
            name: Self::PARAM_NAME.into(),
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
        let result = match interaction {
            Interaction::Ping(_) => todo!(),
            Interaction::ApplicationCommand(comm) => self.on_app_comm(*comm).await,
            Interaction::MessageComponent(msg) => self.on_msg_interaction(*msg).await,
            _ => Err(Error::UnsupportedInteraction),
        };

        let text = match result {
            Ok(res) => return res,
            Err(err) => err.to_string(),
        };

        InteractionResponse::ChannelMessageWithSource(CallbackData {
            content: Some(text),
            flags: Some(MessageFlags::EPHEMERAL),
            tts: None,
            allowed_mentions: None,
            components: None,
            embeds: None,
        })
    }

    /// Responds to new application commands.
    async fn on_app_comm(&self, mut comm: ApplicationCommand) -> Result<InteractionResponse> {
        let user = comm.user.ok_or(Error::UnknownUser)?.id;

        if comm.data.id != self.command {
            return Err(Error::UnknownCommandId);
        }

        if comm.data.name.as_str() != Self::CREATE_NAME {
            return Err(Error::UnknownCommandName);
        }

        // NOTE: We pop off the values to attain O(1) removal time.
        // This does mean that the validation will fail if there are more
        // than one arguments supplied. This should be alright for now since
        // we don't expect the `create` command to accept more than one argument.
        let (name, value) = match comm.data.options.pop() {
            Some(CommandDataOption {
                name,
                value: CommandOptionValue::String(value),
                ..
            }) => (name, value),
            _ => return Err(Error::InvalidParams),
        };

        if name.as_str() != Self::PARAM_NAME {
            return Err(Error::UnknownParamName);
        }

        drop(name);
        let uri = value.parse().map_err(|_| Error::InvalidUri)?;
        drop(value);

        let body = self.http.get(uri).await.map_err(|_| Error::FailedFetch)?.into_body();
        let buf = body::aggregate(body).await?.reader();
        let Quiz { question, choices, .. } = serde_json::from_reader(buf)?;

        // TODO: Spawn external Tokio task for handling incoming responses.

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
        let components = Vec::from([Component::ActionRow(ActionRow {
            components: Vec::from([Component::SelectMenu(SelectMenu {
                options,
                custom_id: String::from("choices"),
                placeholder: Some(String::from("Your Selection")),
                disabled: false,
                min_values: None,
                max_values: None,
            })]),
        })]);
        Ok(InteractionResponse::ChannelMessageWithSource(CallbackData {
            content: Some(question),
            components: Some(components),
            flags: None,
            tts: None,
            allowed_mentions: None,
            embeds: None,
        }))
    }

    /// Responds to message component interactions.
    async fn on_msg_interaction(&self, msg: MessageComponentInteraction) -> Result<InteractionResponse> {
        todo!()
    }
}
