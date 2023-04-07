#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use serde::{Deserialize, Serialize};

pub use uuid::Uuid;

/// Acceptable schema for new questions.
#[derive(Deserialize, Serialize)]
pub struct Quiz {
    /// Question to be displayed in chat.
    pub question: Box<str>,
    /// Possible answers to select from.
    pub choices: Box<[Box<str>]>,
    /// Index of the selection with the correct answer.
    pub answer: Option<u32>,
    /// How long to wait before expiring the poll (in seconds).
    pub timeout: u32,
}
