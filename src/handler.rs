//! # Panic
//! Must be called in a Tokio context.

use crate::{http::Fetcher, model::discord::InteractionCallbackData};
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

    pub fn answer_quiz(&self, user_id: u64, id: usize) -> InteractionCallbackData {
        let lock = self.quiz_channels.read();
        let channel = match lock.get(id) {
            Some(channel) => channel,
            _ => return InteractionCallbackData::MISSING_QUIZ,
        };

        if channel.send((id, user_id)).is_ok() {
            InteractionCallbackData::FOUND_QUIZ
        } else {
            InteractionCallbackData::EXPIRED_QUIZ
        }
    }
}
