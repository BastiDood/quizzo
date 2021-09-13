use serde::Deserialize;

#[derive(Deserialize)]
pub struct Quiz<'a> {
    pub question: &'a str,
    pub answer: usize,
    pub choices: Box<[&'a str]>,
    pub timeout: u64,
}
