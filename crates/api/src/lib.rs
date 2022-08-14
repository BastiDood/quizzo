#![no_std]
extern crate alloc;

mod auth;
mod interaction;
mod lobby;
mod quiz;
mod util;

use alloc::string::String;
use auth::{
    callback::{self, CodeExchanger},
    login::{self, Redirect},
};
use db::Database;
use hyper::{Body, Request, Response, StatusCode};
use lobby::Lobby;
use parking_lot::Mutex;
use rand_core::{CryptoRng, RngCore};
use ring::signature::UnparsedPublicKey;
use twilight_model::id::{marker::ApplicationMarker, Id};

pub use db::{MongoClient, MongoDb, ObjectId};
pub use hyper::Uri;
pub type ApplicationId = Id<ApplicationMarker>;

type HttpClient = hyper::Client<hyper_trust_dns::RustlsHttpsConnector>;

pub struct App<Rng, Bytes>
where
    Bytes: AsRef<[u8]>,
{
    /// Random number generator for cryptographic nonces.
    rng: Mutex<Rng>,
    /// Handle to the database collections.
    db: Database,
    /// Controls for the lobby.
    lobby: Lobby,
    /// Redirects requests to the OAuth consent page.
    redirector: Redirect,
    /// Exchanges authorization codes for token responses.
    exchanger: CodeExchanger,
    /// HTTPS/1.0 client for token-related API calls.
    http: HttpClient,
    /// Public key of the Discord application.
    public: UnparsedPublicKey<Bytes>,
}

impl<Rng, Bytes> App<Rng, Bytes>
where
    Rng: RngCore + CryptoRng,
    Bytes: AsRef<[u8]>,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rand: Rng,
        db: &MongoDb,
        bot_token: String,
        app_id: ApplicationId,
        pub_key: Bytes,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Self {
        use ring::signature::ED25519;
        let connector = hyper_trust_dns::TrustDnsResolver::default().into_rustls_native_https_connector();
        let http = hyper::Client::builder().http1_max_buf_size(8192).set_host(false).build(connector);
        Self {
            http,
            rng: Mutex::new(rand),
            db: Database::new(db),
            lobby: Lobby::new(bot_token, app_id),
            exchanger: CodeExchanger::new(client_id, client_secret, redirect_uri),
            redirector: Redirect::new(client_id, redirect_uri),
            public: UnparsedPublicKey::new(&ED25519, pub_key),
        }
    }

    pub async fn try_respond(&self, req: Request<Body>) -> Result<Response<Body>, StatusCode> {
        use hyper::{http::request::Parts, Method};
        let Self { rng, db, lobby, redirector, exchanger, http, public } = self;
        let (Parts { uri, method, mut headers, .. }, body) = req.into_parts();
        match method {
            Method::GET => match uri.path() {
                "/auth/login" => {
                    // TODO: Verify whether a session already exists.
                    let nonce = rng.lock().next_u64();
                    login::try_respond(nonce, db, redirector).await
                }
                "/auth/callback" => {
                    let query = uri.query().ok_or(StatusCode::BAD_REQUEST)?;
                    callback::try_respond(&headers, query, exchanger, db, http).await
                }
                _ => Err(StatusCode::NOT_FOUND),
            },
            Method::POST => match uri.path() {
                "/discord" => interaction::try_respond(body, &headers, public, db, lobby).await,
                "/quiz" => quiz::try_respond(body, &mut headers, db).await,
                _ => Err(StatusCode::NOT_FOUND),
            },
            Method::PUT | Method::DELETE | Method::PATCH => Err(StatusCode::METHOD_NOT_ALLOWED),
            _ => Err(StatusCode::NOT_IMPLEMENTED),
        }
    }
}
