#![no_std]
extern crate alloc;

use alloc::{string::String, vec::Vec};
use serde::{Serialize, Deserialize};

/// Acceptable schema for new questions.
#[derive(Serialize, Deserialize)]
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
