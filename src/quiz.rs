use serde::Deserialize;

/// Acceptable schema for new questions.
#[derive(Deserialize)]
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
