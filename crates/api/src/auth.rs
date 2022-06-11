use alloc::boxed::Box;
use hyper::{
    header::{HeaderValue, InvalidHeaderValue, CONTENT_TYPE},
    Body, Request, Response, StatusCode, Uri,
};

pub struct Redirect(Box<str>);

impl Redirect {
    pub fn new(id: &str, redirect: &Uri) -> Self {
        let form = alloc::format!(
            "https://discord.com/api/oauth2/authorize?response_type=code&scope=identify&client_id={id}&redirect_uri={redirect}&state="
        );
        Self(form.into_boxed_str())
    }

    pub fn try_respond(&self, state: &str) -> Result<Response<Body>, InvalidHeaderValue> {
        let uri = self.0.clone().into_string() + state;
        let (mut parts, body) = Response::new(Body::empty()).into_parts();
        parts.status = StatusCode::FOUND;
        parts.headers.insert("Location", HeaderValue::from_str(&uri)?);
        Ok(Response::from_parts(parts, body))
    }
}

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
    pub fn new(id: &str, secret: &str, redirect_uri: &Uri) -> Self {
        let form = alloc::format!(
            "grant_type=authorization_code&client_id={id}&client_secret={secret}&redirect_uri={redirect_uri}&code="
        );
        Self(form.into_boxed_str())
    }

    pub fn generate_token_request<'q>(&self, query: &'q str) -> (Request<Body>, &'q str) {
        let (code, state) = parse_code_and_state(query).unwrap();
        let full = self.0.clone().into_string() + code;

        let mut builder = Request::post("https://discord.com/api/oauth2/token");
        let headers = builder.headers_mut().unwrap();
        assert!(!headers.append(CONTENT_TYPE, HeaderValue::from_static("application/x-www-form-urlencoded"),));

        let body = full.into_bytes().into();
        (builder.body(body).unwrap(), state)
    }
}
