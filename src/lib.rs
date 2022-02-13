use dashmap::DashMap;
use tokio::sync::mpsc;

use twilight_http::client::InteractionClient;
use twilight_model::id::{marker::InteractionMarker, Id};

type Key = Id<InteractionMarker>;
type Channel = mpsc::Sender<()>;

pub struct Lobby<'client> {
    polls: DashMap<Key, Channel>,
    client: InteractionClient<'client>,
}

impl<'c> From<InteractionClient<'c>> for Lobby<'c> {
    fn from(client: InteractionClient<'c>) -> Self {
        Self {
            client,
            polls: DashMap::new(),
        }
    }
}
