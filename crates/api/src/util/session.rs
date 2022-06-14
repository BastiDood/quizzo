use hyper::{HeaderMap, StatusCode};

pub fn extract_session(headers: &HeaderMap) -> Result<&str, StatusCode> {
    headers
        .get("Cookie")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .split(';')
        .filter_map(|section| section.split_once('='))
        .find_map(|(key, session)| if key == "sid" { Some(session) } else { None })
        .ok_or(StatusCode::BAD_REQUEST)
}
