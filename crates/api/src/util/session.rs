use db::ObjectId;
use hyper::{HeaderMap, StatusCode};

/// Extracts the session ID from a map of headers.
pub fn extract_session(headers: &HeaderMap) -> Result<&[u8], StatusCode> {
    headers
        .get("Cookie")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .as_bytes()
        .split(|&byte| byte == b';')
        .filter_map(|section| {
            let mid = section.iter().copied().position(|byte| byte == b'=')?;
            let (left, right) = section.split_at(mid);
            let session = &right[1..];
            Some((left, session))
        })
        .find_map(|(key, session)| (key == b"sid").then_some(session))
        .ok_or(StatusCode::UNAUTHORIZED)
}

/// First creates a "salted session" by appending the session ID with a nonce.
/// The result is then hashed with the Blake3 hashing algorithm. This function
/// returns the resulting [`Hasher`](blake3::Hasher). See the linked documentation
/// for more details on retrieving the digest.
pub fn hash_session_salted_with_nonce(session: ObjectId, nonce: u64) -> blake3::Hasher {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&session.bytes()).update(&nonce.to_be_bytes());
    hasher
}
