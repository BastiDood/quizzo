use hyper::{
    body::{to_bytes, Bytes},
    client::HttpConnector,
    http::{self, uri::InvalidUri},
    Body, Client, Request, Uri,
};
use hyper_tls::HttpsConnector;

pub enum FetchError {
    Hyper(hyper::Error),
    Http(hyper::http::Error),
    Uri(InvalidUri),
}

impl From<http::Error> for FetchError {
    fn from(err: http::Error) -> Self {
        Self::Http(err)
    }
}

impl From<hyper::Error> for FetchError {
    fn from(err: hyper::Error) -> Self {
        Self::Hyper(err)
    }
}

impl From<InvalidUri> for FetchError {
    fn from(err: InvalidUri) -> Self {
        Self::Uri(err)
    }
}

pub struct Fetcher {
    webhook_prefix: Box<str>,
    client: Client<HttpsConnector<HttpConnector>>,
}

impl Fetcher {
    pub fn new(application_id: &str) -> Self {
        let mut https = HttpsConnector::new();
        https.https_only(true);
        let client = Client::builder().build(https);
        let webhook_prefix = format!("https://discord.com/api/webhooks/{}/", application_id).into_boxed_str();
        Self { webhook_prefix, client }
    }

    pub async fn get(&self, uri: Uri) -> Result<Bytes, FetchError> {
        let body = self.client.get(uri).await?.into_body();
        let bytes = to_bytes(body).await?;
        Ok(bytes)
    }

    pub async fn create_followup_message(&self, token: &str, content: &str) -> Result<Bytes, FetchError> {
        let uri: Uri = [self.webhook_prefix.as_ref(), token].concat().parse()?;
        let body: Body = format!("{{\"content\":\"{}\"}}", content).into_bytes().into();
        let req = Request::post(uri).body(body)?;
        let response = self.client.request(req).await?.into_body();
        let bytes = to_bytes(response).await?;
        Ok(bytes)
    }
}
