mod bot;

use bot::Bot;
use core::num::NonZeroU64;
use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming},
    HeaderMap, Method, Response, StatusCode,
};

pub use db::{Client, Config, Database, NoTls};
pub use ed25519_dalek::VerifyingKey;

pub struct App {
    /// Command handler.
    bot: Bot,
    /// Ed25519 public key.
    public: VerifyingKey,
}

impl App {
    pub fn new(db: Database, id: NonZeroU64, token: String, public: VerifyingKey) -> Self {
        Self { bot: Bot::new(db, id, token), public }
    }

    pub async fn try_respond(
        &self,
        response: &mut Response<Full<Bytes>>,
        method: Method,
        path: &str,
        headers: HeaderMap,
        mut body: Incoming,
    ) -> bool {
        match method {
            Method::GET | Method::HEAD => match path {
                "/health" => {
                    log::info!("health check pinged");
                    return true;
                }
                _ => {
                    log::error!("unexpected `{method} {path}` request received");
                    *response.status_mut() = StatusCode::NOT_FOUND;
                    return false;
                }
            },
            Method::POST => match path {
                "/discord" => (),
                _ => {
                    log::error!("unexpected `POST {path}` request received");
                    *response.status_mut() = StatusCode::NOT_FOUND;
                    return false;
                }
            },
            _ => {
                log::error!("unexpected `{method} {path}` request received");
                *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
                return false;
            }
        }

        log::debug!("new Discord interaction received");

        // Retrieve security headers
        let signature = headers.get("X-Signature-Ed25519");
        let timestamp = headers.get("X-Signature-Timestamp");
        let Some((signature, timestamp)) = signature.zip(timestamp) else {
            log::error!("no signatures in headers");
            *response.status_mut() = StatusCode::UNAUTHORIZED;
            return false;
        };

        let mut buffer = [0; 64];
        if let Err(err) = hex::decode_to_slice(signature, &mut buffer) {
            log::error!("bad signature hex encoding: {err}");
            *response.status_mut() = StatusCode::BAD_REQUEST;
            return false;
        }

        // Append body after the timestamp
        use http_body_util::BodyExt;
        let mut message = timestamp.as_bytes().to_vec();
        let start = message.len();
        while let Some(frame) = body.frame().await {
            let frame = match frame {
                Ok(frame) => frame,
                Err(err) => {
                    log::error!("body stream prematurely ended: {err}");
                    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                    return false;
                }
            };
            if let Some(data) = frame.data_ref() {
                message.extend_from_slice(data);
            }
        }

        log::debug!("fully received payload body");

        // Validate the challenge
        let signature = ed25519_dalek::Signature::from_bytes(&buffer);
        if let Err(err) = self.public.verify_strict(&message, &signature) {
            log::error!("cannot verify message with signature: {err}");
            *response.status_mut() = StatusCode::FORBIDDEN;
            return false;
        }

        let Some(payload) = message.get(start..) else {
            log::error!("body is empty");
            *response.status_mut() = StatusCode::BAD_REQUEST;
            return false;
        };

        let reply = match serde_json::from_slice(payload) {
            Ok(interaction) => self.bot.on_message(interaction).await,
            Err(err) => {
                log::error!("body is not JSON-encoded: {err}");
                *response.status_mut() = StatusCode::BAD_REQUEST;
                return false;
            }
        };

        *response.body_mut() = match serde_json::to_vec(&reply) {
            Ok(bytes) => bytes.into(),
            Err(err) => {
                log::error!("cannot encode reply to JSON: {err}");
                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                return false;
            }
        };

        use hyper::header::{HeaderValue, CONTENT_TYPE};
        if let Some(value) = response.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/json")) {
            log::warn!("existing header value: {value:?}");
        }
        true
    }
}
