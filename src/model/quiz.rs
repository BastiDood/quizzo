use serde::{Deserialize, Deserializer};
use std::time::Duration;

fn deserialize_as_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let secs = u64::deserialize(deserializer)?;
    Ok(Duration::from_secs(secs))
}

#[derive(Deserialize)]
pub struct Quiz<'a> {
    pub question: &'a str,
    pub answer: usize,
    pub choices: Box<[&'a str]>,
    #[serde(deserialize_with = "deserialize_as_duration")]
    pub timeout: Duration,
}
