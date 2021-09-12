//! # Panic
//! Must be called in a Tokio context.

use crate::{
    http::Fetcher,
    model::{Interaction, InteractionData},
};
use parking_lot::RwLock;
use slab::Slab;
use tokio::sync::mpsc::UnboundedSender;

type AnswerAndUser = (usize, u64);
type PendingQuiz = UnboundedSender<AnswerAndUser>;

pub struct Handler {
    client: Fetcher,
    quiz_channels: RwLock<Slab<PendingQuiz>>,
}

impl Handler {
    pub fn on_interaction(interaction: Interaction) {
        todo!()
    }
}
