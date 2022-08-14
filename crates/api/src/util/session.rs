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

/// Appends the session ID with a nonce.
///
/// # Panic
/// If the lengths of the session and the nonce do not add up to 20, the function will panic.
/// More specifically, the function expects that [`ObjectId`](ObjectId) contains 12 bytes
/// internally. There are 8 more bytes allocated for the nonce, which is a [`u64`](u64). Therefore,
/// the salt must have a length of 12 + 8 = 20.
pub fn salt_session_with_nonce(session: ObjectId, nonce: u64) -> [u8; 20] {
    let session_bytes = session.bytes();
    let nonce_bytes = nonce.to_be_bytes();

    let mut salted = [0; 20];
    let (left, right) = salted.split_at_mut(session_bytes.len());
    left.copy_from_slice(&session_bytes);
    right.copy_from_slice(&nonce_bytes);
    salted
}
