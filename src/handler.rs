//! # Panic
//! Must be called in a Tokio context.

use crate::{
    http::{FetchError, Fetcher},
    model::{discord::InteractionCallbackData, quiz::Quiz},
};
use parking_lot::RwLock;
use slab::Slab;
use std::{borrow::Cow, num::NonZeroU64, sync::Arc};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

type AnswerAndUser = (usize, u64);
type PendingQuiz = UnboundedSender<AnswerAndUser>;

pub struct QuizHandler {
    command_id: NonZeroU64,
    quiz_channels: RwLock<Slab<PendingQuiz>>,
}

impl QuizHandler {
    pub async fn create_quiz(self: Arc<Self>, mut ctx: Fetcher, url: &str) -> InteractionCallbackData<'static> {
        let Quiz {
            question,
            answer,
            choices,
            timeout,
        } = match ctx.retrieve_quiz(url).await {
            Ok(quiz) => quiz,
            Err(FetchError::Http(_) | FetchError::Hyper(_)) => return InteractionCallbackData::FETCH_ERROR,
            Err(FetchError::Uri(_)) => return InteractionCallbackData::MALFORMED_URL,
            Err(FetchError::Io(_)) => return InteractionCallbackData::MALFORMED_QUIZ,
        };

        let (tx, rx) = unbounded_channel();
        let key = self.quiz_channels.write().insert(tx);
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
