#![no_std]

extern crate alloc;

mod auth;
mod quiz;

use alloc::{string::String, vec::Vec};
use auth::{CodeExchanger, Redirect};
use db::Database;
use hyper::{Body, Request, Response, StatusCode};

pub use db::{MongoClient, MongoDb, ObjectId};
pub use hyper::Uri;

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
        let http = hyper::Client::builder()
            .http1_max_buf_size(8192)
            .set_host(false)
            .build(connector);
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
        let (
            Parts {
                uri, method, headers, ..
            },
            body,
        ) = req.into_parts();
        match (method, uri.path()) {
            (Method::POST, "/discord") => todo!(),
            (Method::POST, "/quiz") => {
                // Retrieve the session from the cookie
                let (key, session) = headers
                    .get("Cookie")
                    .ok_or(StatusCode::UNAUTHORIZED)?
                    .to_str()
                    .map_err(|_| StatusCode::BAD_REQUEST)?
                    .split(';')
                    .next()
                    .ok_or(StatusCode::BAD_REQUEST)?
                    .split_once('=')
                    .ok_or(StatusCode::BAD_REQUEST)?;
                if key != "sid" {
                    return Err(StatusCode::BAD_REQUEST);
                }

                // Check database if user ID is present
                let oid = ObjectId::parse_str(session).map_err(|_| StatusCode::BAD_REQUEST)?;
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
                let reader = body::aggregate(body)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                    .reader();
                let submission: Quiz =
                    serde_json::from_reader(reader).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                todo!()
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
                self.redirector
                    .try_respond(hash_str)
                    .map_err(|_| StatusCode::BAD_REQUEST)
            }
            (Method::GET, "/auth/callback") => todo!(),
            _ => Err(StatusCode::NOT_FOUND),
        }
    }
}
