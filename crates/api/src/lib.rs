mod bot;

use bot::Bot;
use core::num::NonZeroU64;
use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming},
    Request, Response, StatusCode,
};
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

    pub async fn try_respond(&self, req: Request<Incoming>) -> Result<Response<Full<Bytes>>, StatusCode> {
        use hyper::{http::request::Parts, Method};
        let (Parts { uri, method, headers, .. }, mut body) = req.into_parts();
        let path = uri.path();

        match method {
            Method::GET | Method::HEAD => match path {
                "/health" => {
                    log::info!("Health check pinged");
                    return Ok(Default::default());
                }
                _ => {
                    log::error!("Unexpected `{method} {path}` request received");
                    return Err(StatusCode::NOT_FOUND);
                }
            },
            Method::POST => match path {
                "/discord" => (),
                _ => {
                    log::error!("Unexpected `POST {path}` request received");
                    return Err(StatusCode::NOT_FOUND);
                }
            },
            _ => {
                log::error!("Unexpected `{method} {path}` request received");
                return Err(StatusCode::METHOD_NOT_ALLOWED);
            }
        }

        // Retrieve security headers
        log::debug!("New Discord interaction received");
        let signature = headers.get("X-Signature-Ed25519");
        let timestamp = headers.get("X-Signature-Timestamp");
        let (sig, timestamp) = signature.zip(timestamp).ok_or(StatusCode::UNAUTHORIZED)?;
        let mut signature = [0; 64];
        hex::decode_to_slice(sig, &mut signature).map_err(|_| StatusCode::BAD_REQUEST)?;

        // Append body after the timestamp
        use http_body_util::BodyExt;
        let mut message = timestamp.as_bytes().to_vec();
        let start = message.len();
        while let Some(frame) = body.frame().await {
            let frame = match frame {
                Ok(frame) => frame,
                Err(err) => {
                    log::error!("body stream prematurely ended: {err}");
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            };
            if let Some(data) = frame.data_ref() {
                message.extend_from_slice(data);
            }
        }
        log::debug!("Fully received payload body.");

        // Validate the challenge
        self.public.verify(&message, &signature).map_err(|_| StatusCode::UNAUTHORIZED)?;

        // Parse incoming interaction
        let payload = message.get(start..).ok_or(StatusCode::BAD_REQUEST)?;
        let interaction = serde_json::from_slice(payload).map_err(|_| StatusCode::BAD_REQUEST)?;

        // Construct new body
        let reply = self.bot.on_message(interaction).await;
        let bytes = serde_json::to_vec(&reply).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        use hyper::header::{HeaderValue, CONTENT_TYPE};
        let mut res = Response::new(Full::from(bytes));
        let result = res.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        assert!(result.is_none());
        Ok(res)
    }
}
