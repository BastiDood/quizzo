use dashmap::DashMap;
use hyper::{client::HttpConnector, Client};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
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
    polls: DashMap<Key, Channel>,
    /// Discord API interactions.
    api: InteractionClient<'client>,
    /// Arbitrary HTTP fetching of JSON files.
    http: Client<HttpsConnector<HttpConnector>>,
}

impl<'c> From<InteractionClient<'c>> for Lobby<'c> {
    fn from(api: InteractionClient<'c>) -> Self {
        let connector = HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_only()
            .enable_http2()
            .build();
        let http = Client::builder().http2_only(true).build(connector);
        Self {
            api,
            http,
            polls: DashMap::new(),
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
