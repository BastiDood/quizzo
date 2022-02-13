use serde::Deserialize;

/// Acceptable schema for new questions.
#[derive(Deserialize)]
pub struct Quiz {
    /// Question to be displayed in chat.
    question: Box<str>,
    /// Possible answers to select from.
    choices: Box<[Box<str>]>,
    /// Index of the selection with the correct answer.
    answer: u8,
    /// How long to wait before expiring the poll (in seconds).
    timeout: u8,
}
