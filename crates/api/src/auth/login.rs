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

    log::info!("Created new session: {}", oid);

    // Encode session ID to hex (to be used as the cookie)
    let mut orig_buf = [0; 12 * 2];
    hex::encode_to_slice(oid.bytes(), &mut orig_buf).unwrap();
    let orig_hex = core::str::from_utf8(&orig_buf).unwrap();

    // Hash the salted session ID
    let hash = crate::util::session::hash_session_salted_with_nonce(oid, nonce).finalize().to_hex();
    log::info!("Derived the hash for salted session: {}", hash);

    use hyper::header::{HeaderValue, LOCATION, SET_COOKIE};
    let mut res = Response::new(Body::empty());
    *res.status_mut() = StatusCode::FOUND;
    let headers = res.headers_mut();

    let redirect = redirector.generate_consent_page_uri(hash.as_str());
    let location = HeaderValue::from_str(&redirect).unwrap();
    assert!(!headers.append(LOCATION, location));

    let cookie_str = alloc::format!("sid={orig_hex}; Secure; HttpOnly; SameSite=Lax; Max-Age=900");
    let cookie = HeaderValue::from_str(&cookie_str).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    assert!(!headers.append(SET_COOKIE, cookie));

    Ok(res)
}
