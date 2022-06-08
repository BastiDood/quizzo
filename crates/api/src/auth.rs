use hyper::{header::{HeaderValue, CONTENT_TYPE}, Body, Request, Response, StatusCode, Uri};

pub fn create_redirect_responder(client_id: &str, redirect_uri: &Uri) -> impl Fn(&str) -> Response<()> {
    let form = alloc::format!(
        "https://discord.com/api/oauth2/authorize?response_type=code&scope=identify&client_id={client_id}&redirect_uri={redirect_uri}&state="
    );
    move |session| {
        let uri = form.clone() + session;
        let (mut parts, body) = Response::new(()).into_parts();
        parts.status = StatusCode::FOUND;
        parts.headers.insert("Location", HeaderValue::from_str(&uri).unwrap());
        Response::from_parts(parts, body)
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

pub fn exchange_code_for_token<'q>(
    id: &str,
    secret: &str,
    redirect_uri: &Uri,
) -> impl Fn(&'q str) -> (Request<Body>, &'q str) {
    let form = alloc::format!(
        "grant_type=authorization_code&client_id={id}&client_secret={secret}&redirect_uri={redirect_uri}&code="
    );
    move |query| {
        let (code, state) = parse_code_and_state(query).unwrap();
        let full = form.clone() + code;

        let mut builder = Request::post("https://discord.com/api/oauth2/token");
        let headers = builder.headers_mut().unwrap();
        assert!(!headers.append(
            CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        ));

        let body = full.into_bytes().into();
        (builder.body(body).unwrap(), state)
    }
}
