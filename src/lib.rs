mod quiz;

use dashmap::DashMap;
use hyper::Client;
use hyper_trust_dns::RustlsHttpsConnector;
use quiz::Quiz;
use tokio::sync::mpsc;

use twilight_http::client::InteractionClient;
use twilight_model::{
    application::interaction::{ApplicationCommand, MessageComponentInteraction},
    id::{marker::InteractionMarker, Id},
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
}

impl<'c> From<InteractionClient<'c>> for Lobby<'c> {
    fn from(api: InteractionClient<'c>) -> Self {
        let connector = hyper_trust_dns::new_rustls_native_https_connector();
        let http = Client::builder().http2_only(true).build(connector);
        Self {
            api,
            http,
            quizzes: DashMap::new(),
        }
    }
}

impl Lobby<'_> {
    /// Responds to new application commands.
    pub async fn on_app_comm(&self, comm: ApplicationCommand) {
        todo!()
    }

    /// Responds to message component interactions.
    pub async fn on_msg_interaction(&self, comm: MessageComponentInteraction) {
        todo!()
    }
}
