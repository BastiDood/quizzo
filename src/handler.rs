//! # Panic
//! Must be called in a Tokio context.

use crate::{
    http::Fetcher,
    model::discord::{Interaction, InteractionData, InteractionResponse},
};
use parking_lot::RwLock;
use slab::Slab;
use std::num::NonZeroU64;
use tokio::sync::mpsc::UnboundedSender;

type AnswerAndUser = (usize, u64);
type PendingQuiz = UnboundedSender<AnswerAndUser>;

pub struct Handler {
    command_id: NonZeroU64,
    client: Fetcher,
    quiz_channels: RwLock<Slab<PendingQuiz>>,
}

impl Handler {
    pub fn on_interaction(
        &self,
        Interaction {
            interaction_id,
            application_id,
            user_id,
            data,
            token,
        }: Interaction,
    ) -> InteractionResponse {
        match data {
            InteractionData::Ping => InteractionResponse::Pong,
            InteractionData::AppCommand { url, name, command_id } if command_id == self.command_id => todo!(),
            _ => todo!(),
        }
    }
}
