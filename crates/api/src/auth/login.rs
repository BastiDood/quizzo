use alloc::boxed::Box;
use db::Database;
use hyper::{Body, Response, StatusCode};

pub struct Redirect(Box<str>);

impl Redirect {
    pub fn new(id: &str, redirect: &str) -> Self {
        let form = alloc::format!(
            "https://discord.com/api/oauth2/authorize?response_type=code&scope=identify&client_id={id}&redirect_uri={redirect}&state="
        );
        Self(form.into_boxed_str())
    }

    fn generate_consent_page_uri(&self, state: &str) -> Box<str> {
        let uri = self.0.clone().into_string() + state;
        uri.into_boxed_str()
    }
}

pub async fn try_respond(nonce: u64, db: &Database, redirector: &Redirect) -> Result<Response<Body>, StatusCode> {
    let oid = match db.create_session(nonce).await {
        Ok(oid) => oid,
        Err(db::error::Error::AlreadyExists) => return Err(StatusCode::FORBIDDEN),
        _ => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // Encode session ID to hex (to be used as the cookie)
    let mut orig_buf = [0; 12 * 2];
    hex::encode_to_slice(&oid.bytes(), &mut orig_buf).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let orig_hex = core::str::from_utf8(&orig_buf).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Hash the salted session ID
    use ring::digest;
    let salted = crate::util::session::salt_session_with_nonce(oid, nonce);
    let mut hash_buf = [0; 32 * 2];
    let hash = digest::digest(&digest::SHA256, &salted);
    hex::encode_to_slice(hash, &mut hash_buf).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let hash_str = core::str::from_utf8(hash_buf.as_slice()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    use hyper::header::{HeaderValue, LOCATION, SET_COOKIE};
    let mut res = Response::new(Body::empty());
    *res.status_mut() = StatusCode::FOUND;
    let headers = res.headers_mut();

    let redirect = redirector.generate_consent_page_uri(hash_str);
    let location = HeaderValue::from_str(&redirect).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if headers.insert(LOCATION, location).is_some() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let cookie_str = alloc::format!("sid={orig_hex}; Secure; HttpOnly; SameSite=Lax; Max-Age=900");
    let cookie = HeaderValue::from_str(&cookie_str).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if headers.insert(SET_COOKIE, cookie).is_some() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(res)
}
