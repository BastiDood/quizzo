#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use core::num::NonZeroU64;
use serde::{Deserialize, Serialize};

/// Acceptable schema for new questions.
#[derive(Deserialize, Serialize)]
pub struct Quiz<'choices> {
    /// Question to be displayed in chat.
    pub question: Box<str>,
    /// Possible answers to select from.
    #[serde(borrow)]
    pub choices: Box<[&'choices str]>,
    /// Index of the selection with the correct answer.
    pub answer: u8,
    /// How long to wait before expiring the poll (in seconds).
    pub timeout: u8,
}

#[derive(Deserialize, Serialize)]
pub struct Submission<'choices> {
    /// Discord User ID that created this quiz.
    pub id: NonZeroU64,
    #[serde(flatten)]
    /// The actual quiz information.
    #[serde(borrow)]
    pub quiz: Quiz<'choices>,
}
