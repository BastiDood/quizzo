use hyper::{header::HeaderValue, Response, StatusCode, Uri};

pub fn create_redirect_response(client_id: &str, redirect_uri: &Uri) -> impl Fn(&str) -> Response<()> {
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
