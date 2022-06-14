use db::ObjectId;
use hyper::{HeaderMap, StatusCode};

/// Extracts the sessionn ID from a map of headers.
pub fn extract_session(headers: &HeaderMap) -> Result<&str, StatusCode> {
    headers
        .get("Cookie")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .split(';')
        .filter_map(|section| section.split_once('='))
        .find_map(|(key, session)| if key == "sid" { Some(session) } else { None })
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
