#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use core::num::NonZeroI16;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RawQuiz {
    /// Question to be displayed in chat.
    pub question: Box<str>,
    /// Possible answers to select from.
    pub choices: Box<[Box<str>]>,
    /// Index of the selection with the correct answer.
    pub answer: Option<u32>,
    /// How long to wait before expiring the poll (in seconds).
    pub expiration: u32,
}

#[derive(Deserialize)]
pub struct Quiz {
    /// Monotonically increasing quiz ID.
    pub id: NonZeroI16,
    /// The raw internal quiz.
    #[serde(flatten)]
    pub raw: RawQuiz,
}
