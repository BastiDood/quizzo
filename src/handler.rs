//! # Panic
//! Must be called in a Tokio context.

use crate::model::{Interaction, InteractionData};
use parking_lot::RwLock;
use slab::Slab;
use tokio::sync::mpsc::UnboundedSender;

type AnswerAndUser = (usize, u64);

pub struct Handler {
    quiz_channels: RwLock<Slab<UnboundedSender<AnswerAndUser>>>,
}

impl Handler {
    pub fn on_interaction(interaction: Interaction) {
        todo!()
    }
}
