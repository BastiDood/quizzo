use core::num::NonZeroU64;

use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

/// Acceptable schema for new questions.
#[derive(Deserialize, Serialize)]
pub struct Quiz {
    /// Question to be displayed in chat.
    pub question: String,
    /// Possible answers to select from.
    pub choices: Vec<String>,
    /// Index of the selection with the correct answer.
    pub answer: u8,
    /// How long to wait before expiring the poll (in seconds).
    pub timeout: u8,
}

#[derive(Deserialize, Serialize)]
pub struct Submission {
    /// Discord User ID that created this quiz.
    #[serde(rename = "_id")]
    pub id: NonZeroU64,
    #[serde(flatten)]
    /// The actual quiz information.
    pub quiz: Quiz,
}
