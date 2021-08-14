use serde::Deserialize;

#[derive(Deserialize)]
pub struct Quiz<'a> {
    pub question: &'a str,
    pub answer: usize,
    pub choices: Vec<&'a str>,
    pub timeout: u64,
}
