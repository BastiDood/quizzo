#![no_std]

extern crate alloc;

mod auth;
mod quiz;

use alloc::{string::String, vec::Vec};
use auth::{CodeExchanger, Redirect};
use db::Database;
use hyper::{Body, HeaderMap, Request, Response, StatusCode};

pub use db::{MongoClient, MongoDb, ObjectId};
pub use hyper::Uri;
use model::{oauth::TokenResponse, quiz::Submission};

pub struct App {
    /// Handle to the database collections.
    db: Database,
    /// Wrapper for the Discord API bot.
    discord: twilight_http::Client,
    /// Redirects requests to the OAuth consent page.
    redirector: Redirect,
    /// Exchanges authorization codes for token responses.
    exchanger: CodeExchanger,
    /// HTTPS/1.0 client for token-related API calls.
    http: hyper::Client<hyper_trust_dns::RustlsHttpsConnector>,
}

impl App {
    pub fn new(db: &MongoDb, bot_token: String, client_id: &str, client_secret: &str, redirect_uri: &Uri) -> Self {
        let connector = hyper_trust_dns::TrustDnsResolver::default().into_rustls_native_https_connector();
        let http = hyper::Client::builder().http1_max_buf_size(8192).set_host(false).build(connector);
        Self {
            db: Database::new(db),
            discord: twilight_http::Client::new(bot_token),
            exchanger: CodeExchanger::new(client_id, client_secret, redirect_uri),
            redirector: Redirect::new(client_id, redirect_uri),
            http,
        }
    }

    pub async fn try_respond(&self, req: Request<Body>) -> Result<Response<Body>, StatusCode> {
        use hyper::{body, http::request::Parts, Method};
        let (Parts { uri, method, headers, .. }, body) = req.into_parts();
        match (method, uri.path()) {
            (Method::POST, "/discord") => todo!(),
            (Method::POST, "/quiz") => {
                // Retrieve the session from the cookie
                let session = extract_session(&headers)?;
                let oid = ObjectId::parse_str(session).map_err(|_| StatusCode::BAD_REQUEST)?;

                // Check database if user ID is present
                let user = self
                    .db
                    .get_session(oid)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                    .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
                    .ok_or(StatusCode::FORBIDDEN)?;

                // Finally parse the JSON form submission
                use body::Buf;
                use model::quiz::Quiz;
                let reader = body::aggregate(body).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.reader();
                let quiz: Quiz = serde_json::from_reader(reader).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                // Submit the quiz to the database
                let submission = Submission { id: user, quiz };
                let oid: Vec<_> = quiz::try_submit_quiz(&self.db, &submission).await?.into();
                let mut res = Response::new(oid.into());
                *res.status_mut() = StatusCode::CREATED;
                Ok(res)
            }
            (Method::GET, "/auth/login") => {
                // TODO: Verify whether a session already exists.

                use ring::digest;
                let oid = match self.db.create_session().await {
                    Ok(oid) => oid.bytes(),
                    Err(db::error::Error::AlreadyExists) => return Err(StatusCode::FORBIDDEN),
                    _ => return Err(StatusCode::INTERNAL_SERVER_ERROR),
                };

                assert_eq!(oid.len(), 12);
                let mut buf = [0; 12 * 2];
                let hash = digest::digest(&digest::SHA256, &oid);
                hex::encode_to_slice(hash, &mut buf).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                let hash_str = core::str::from_utf8(buf.as_slice()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                self.redirector.try_respond(hash_str).map_err(|_| StatusCode::BAD_REQUEST)
            }
            (Method::GET, "/auth/callback") => {
                // Retrieve the session from the cookie
                let session = extract_session(&headers)?;
                let oid = ObjectId::parse_str(session).map_err(|_| StatusCode::BAD_REQUEST)?;

                let query = uri.query().ok_or(StatusCode::BAD_REQUEST)?;
                let (req, state) = self.exchanger.generate_token_request(query).ok_or(StatusCode::BAD_REQUEST)?;
                let body = self.http.request(req).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.into_body();

                use body::Buf;
                let reader = body::aggregate(body).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.reader();
                let TokenResponse { access, refresh, expires } =
                    serde_json::from_reader(reader).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                // TODO: store OAuth tokens somewhere in the database

                use twilight_model::user::CurrentUser;
                let client = twilight_http::Client::new(access.into_string());
                let CurrentUser { id, .. } = client
                    .current_user()
                    .exec()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                    .model()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                self.db.upgrade_session(oid, id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                use hyper::header::HeaderValue;
                let mut res = Response::new(Body::empty());
                *res.status_mut() = StatusCode::FOUND;
                assert!(res.headers_mut().insert("Location", HeaderValue::from_static("/")).is_none());
                Ok(res)
            }
            (Method::GET, _) => Err(StatusCode::NOT_FOUND),
            (_, "/discord" | "/quiz" | "/auth/login" | "/auth/callback") => Err(StatusCode::METHOD_NOT_ALLOWED),
            _ => Err(StatusCode::NOT_IMPLEMENTED),
        }
    }
}

fn extract_session(headers: &HeaderMap) -> Result<&str, StatusCode> {
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
