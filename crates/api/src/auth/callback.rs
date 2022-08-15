use crate::HttpClient;
use alloc::boxed::Box;
use db::Database;
use hyper::{Body, HeaderMap, Request, Response, StatusCode};

fn parse_code_and_state(query: &str) -> Option<(&str, &str)> {
    let mut code = None;
    let mut state = None;

    for chunk in query.split('&') {
        let (key, value) = match chunk.split_once('=') {
            Some(pair) => pair,
            _ => continue,
        };
        let target = match key {
            "code" => &mut code,
            "state" => &mut state,
            _ => continue,
        };
        *target = Some(value);
    }

    code.zip(state)
}

pub struct CodeExchanger(Box<str>);

impl CodeExchanger {
    pub fn new(id: &str, secret: &str, redirect_uri: &str) -> Self {
        let form = alloc::format!(
            "grant_type=authorization_code&client_id={id}&client_secret={secret}&redirect_uri={redirect_uri}&code="
        );
        Self(form.into_boxed_str())
    }

    fn generate_token_request<'q>(&self, query: &'q str) -> Option<(Request<Body>, &'q str)> {
        let (code, state) = parse_code_and_state(query)?;
        let full = self.0.clone().into_string() + code;

        let body = full.into_bytes().into();
        let mut req = Request::new(body);

        *req.method_mut() = hyper::Method::POST;
        *req.uri_mut() = hyper::Uri::from_static("https://discord.com/api/oauth2/token");

        use hyper::header::{HeaderValue, CONTENT_TYPE};
        assert!(!req.headers_mut().append(CONTENT_TYPE, HeaderValue::from_static("application/x-www-form-urlencoded")));

        Some((req, state))
    }
}

pub async fn try_respond(
    headers: &HeaderMap,
    query: &str,
    exchanger: &CodeExchanger,
    db: &Database,
    http: &HttpClient,
) -> Result<Response<Body>, StatusCode> {
    use crate::util::session;
    let session_str_bytes = crate::util::session::extract_session(headers)?;
    let session = core::str::from_utf8(session_str_bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    let oid = db::ObjectId::parse_str(session).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Check database if user ID is present
    let nonce = db
        .get_session(oid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?
        .as_nonce()
        .ok_or(StatusCode::FORBIDDEN)?;

    // Hash the salted session ID
    let hash = session::hash_session_salted_with_nonce(oid, nonce).finalize();

    // Parse the `state` parameter as raw bytes
    let (req, state) = exchanger.generate_token_request(query).ok_or(StatusCode::BAD_REQUEST)?;
    let mut state_buf = [0; 32];
    hex::decode_to_slice(state, &mut state_buf).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Validate whether the hash of the session matches
    if hash.as_bytes().ne(&state_buf) {
        log::error!("OAuth state does not match with hash.");
        return Err(StatusCode::BAD_REQUEST);
    }

    use hyper::body::{self, Buf};
    let body = http.request(req).await.unwrap().into_body();
    let reader = body::aggregate(body).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.reader();
    let model::oauth::TokenResponse { access, refresh, expires } =
        serde_json::from_reader(reader).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    log::info!("Successfully received OAuth token from Discord.");

    use twilight_model::user::CurrentUser;
    let client = twilight_http::Client::new(access.clone().into_string());
    let CurrentUser { id, .. } = client
        .current_user()
        .exec()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .model()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let expires = core::time::Duration::from_secs(expires.get());
    if !db
        .upgrade_session(oid, id.into_nonzero(), access, refresh, expires)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        log::error!("Cannot upgrade session {} for user {}.", oid, id);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    use hyper::header::{HeaderValue, LOCATION};
    let mut res = Response::new(Body::empty());
    *res.status_mut() = StatusCode::FOUND;
    assert!(!res.headers_mut().append(LOCATION, HeaderValue::from_static("/")));

    Ok(res)
}
