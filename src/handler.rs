//! # Panic
//! Must be called in a Tokio context.

use crate::{
    http::{FetchError, Fetcher},
    model::discord::{Interaction, InteractionData},
};
use parking_lot::RwLock;
use slab::Slab;
use std::{num::NonZeroU64, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;

type AnswerAndUser = (usize, u64);
type PendingQuiz = UnboundedSender<AnswerAndUser>;

pub struct QuizHandler {
    command_id: NonZeroU64,
    quiz_channels: RwLock<Slab<PendingQuiz>>,
}

impl QuizHandler {
    pub fn create_quiz(self: Arc<Self>, mut ctx: Fetcher, url: &str) {
        todo!()
    }
}
