#![no_std]
extern crate alloc;

mod bot;

use alloc::{boxed::Box, string::String};
use bot::Bot;
use core::num::NonZeroU64;
use hyper::{Body, Request, Response, StatusCode};
use ring::signature::UnparsedPublicKey;

pub use db::{Client, Config, Database, NoTls};

pub struct App {
    /// Command handler.
    bot: Bot,
    /// Ed25519 public key.
    public: UnparsedPublicKey<Box<[u8]>>,
}

impl App {
    pub fn new(db: Database, id: NonZeroU64, token: String, public: Box<[u8]>) -> Self {
        Self { bot: Bot::new(db, id, token), public: UnparsedPublicKey::new(&ring::signature::ED25519, public) }
    }

    pub async fn try_respond(&self, req: Request<Body>) -> Result<Response<Body>, StatusCode> {
        use hyper::{http::request::Parts, Method};
        let (Parts { uri, method, headers, .. }, body) = req.into_parts();
        let path = uri.path();

        if method == Method::GET && path == "/healthz" {
            log::info!("Health check pinged");
            return Ok(Response::new(Body::empty()));
        }

        if method != Method::POST {
            log::error!("Non-POST request received");
            return Err(StatusCode::METHOD_NOT_ALLOWED);
        }

        if path != "/discord" {
            log::error!("Non-Discord POST request received");
            return Err(StatusCode::NOT_FOUND);
        }

        // Retrieve security headers
        log::debug!("New Discord interaction received");
        let maybe_sig = headers.get("X-Signature-Ed25519");
        let maybe_time = headers.get("X-Signature-Timestamp");
        let (sig, timestamp) = maybe_sig.zip(maybe_time).ok_or(StatusCode::UNAUTHORIZED)?;

        let mut signature = [0; 64];
        hex::decode_to_slice(sig, &mut signature).map_err(|_| StatusCode::BAD_REQUEST)?;

        // Append body after the timestamp
        let payload = hyper::body::to_bytes(body).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let mut message = timestamp.as_bytes().to_vec();
        message.extend_from_slice(&payload);

        // Validate the challenge
        self.public.verify(&message, &signature).map_err(|_| StatusCode::UNAUTHORIZED)?;
        drop(message);

        // Parse incoming interaction
        let interaction = serde_json::from_slice(&payload).map_err(|_| StatusCode::BAD_REQUEST)?;
        drop(payload);

        // Construct new body
        let reply = self.bot.on_message(interaction).await;
        let bytes = serde_json::to_vec(&reply).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        use hyper::header::{HeaderValue, CONTENT_TYPE};
        let mut res = Response::new(Body::from(bytes));
        assert!(res.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/json")).is_none());
        Ok(res)
    }
}
