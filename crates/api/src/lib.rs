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
    login::Redirect,
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
        let (Parts { uri, method, headers, .. }, body) = req.into_parts();
        match (method, uri.path()) {
            (Method::POST, "/discord") => interaction::try_respond(body, &headers, public, db, lobby).await,
            (Method::POST, "/quiz") => quiz::try_respond(body, &headers, db).await,
            (Method::GET, "/auth/login") => {
                // TODO: Verify whether a session already exists.

                // Create new session with nonce
                let nonce = rng.lock().next_u64();
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
                let salted = util::session::salt_session_with_nonce(oid, nonce);
                let mut hash_buf = [0; 32 * 2];
                let hash = digest::digest(&digest::SHA256, &salted);
                hex::encode_to_slice(hash, &mut hash_buf).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                let hash_str =
                    core::str::from_utf8(hash_buf.as_slice()).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                use hyper::header::{HeaderValue, LOCATION, SET_COOKIE};
                let redirect = redirector.generate_consent_page_uri(hash_str);
                let location = HeaderValue::from_str(&redirect).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                let cookie_str = alloc::format!("sid={orig_hex}; Secure; HttpOnly; SameSite=Lax; Max-Age=900");
                let cookie = HeaderValue::from_str(&cookie_str).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                let mut res = Response::new(Body::empty());
                *res.status_mut() = StatusCode::FOUND;

                let headers = res.headers_mut();
                assert!(headers.insert(LOCATION, location).is_none());
                assert!(headers.insert(SET_COOKIE, cookie).is_none());
                Ok(res)
            }
            (Method::GET, "/auth/callback") => {
                let query = uri.query().ok_or(StatusCode::BAD_REQUEST)?;
                callback::try_respond(&headers, query, exchanger, db, http).await
            }
            (Method::GET, _) => Err(StatusCode::NOT_FOUND),
            (_, "/discord" | "/quiz" | "/auth/login" | "/auth/callback") => Err(StatusCode::METHOD_NOT_ALLOWED),
            _ => Err(StatusCode::NOT_IMPLEMENTED),
        }
    }
}
