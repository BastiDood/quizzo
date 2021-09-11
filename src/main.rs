use hyper::{
    body,
    service::{make_service_fn, service_fn},
    Body, Method, Response, StatusCode, {Error as HyperError, Server},
};
use ring::signature::{UnparsedPublicKey, ED25519};
use std::{
    convert::Infallible,
    env::{self, VarError},
    future,
    io::Error as IoError,
    net::{Ipv4Addr, TcpListener},
    num::NonZeroU64,
    sync::Arc,
};
use tokio::runtime::Builder;

#[derive(Debug)]
enum AppError {
    MissingEnvVars,
    InvalidPublicKey,
    Hyper(HyperError),
    Io(IoError),
}

impl From<VarError> for AppError {
    fn from(_: VarError) -> Self {
        Self::MissingEnvVars
    }
}

impl From<IoError> for AppError {
    fn from(err: IoError) -> Self {
        Self::Io(err)
    }
}

impl From<HyperError> for AppError {
    fn from(err: HyperError) -> Self {
        Self::Hyper(err)
    }
}

fn main() -> Result<(), AppError> {
    // Retrieve environment variables
    let port = env::var("PORT")?
        .parse()
        .map_err(|_| AppError::MissingEnvVars)?;
    let bot_token = env::var("BOT_TOKEN")?;
    let application_id = env::var("APPLICATION_ID")?;
    let guild_id = env::var("GUILD_ID")?
        .parse::<u64>()
        .ok()
        .and_then(NonZeroU64::new);

    // Try to parse public key
    let pub_bytes: Arc<[u8]> = hex::decode(application_id)
        .map_err(|_| AppError::InvalidPublicKey)?
        .into();
    let pub_key = UnparsedPublicKey::new(&ED25519, pub_bytes);

    // Configure main service
    let service = make_service_fn(move |_| {
        let outer_pub_key = pub_key.clone();
        let outer = service_fn(move |req| {
            let inner_pub_key = outer_pub_key.clone();
            async move {
                // Disallow non-POST methods
                if req.method() != Method::POST {
                    let mut res = Response::new(Body::empty());
                    *res.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
                    return Ok(res);
                }

                // Disallow any other route
                if req.uri().path() != "/" {
                    let mut res = Response::new(Body::empty());
                    *res.status_mut() = StatusCode::NOT_FOUND;
                    return Ok(res);
                }

                // Check existence of signatures
                let signature = req
                    .headers()
                    .get("X-Signature-Ed25519")
                    .and_then(|sig| sig.to_str().ok())
                    .unwrap_or_default();
                let timestamp = req
                    .headers()
                    .get("X-Signature-Timestamp")
                    .and_then(|ts| ts.to_str().ok())
                    .unwrap_or_default();
                if signature.is_empty() || timestamp.is_empty() {
                    let mut res = Response::new(Body::empty());
                    *res.status_mut() = StatusCode::UNAUTHORIZED;
                    return Ok(res);
                }

                // Convert hex strings to bytes
                let signature = match hex::decode(signature) {
                    Ok(sig) => sig,
                    _ => {
                        let mut res = Response::new(Body::empty());
                        *res.status_mut() = StatusCode::BAD_REQUEST;
                        return Ok(res);
                    }
                };
                let mut timestamp = match hex::decode(timestamp) {
                    Ok(ts) => ts,
                    _ => {
                        let mut res = Response::new(Body::empty());
                        *res.status_mut() = StatusCode::BAD_REQUEST;
                        return Ok(res);
                    }
                };

                // Verify signatures
                let body = body::to_bytes(req.into_body()).await?;
                timestamp.extend_from_slice(&body);
                if let Err(_) = inner_pub_key.verify(&timestamp, &signature) {
                    let mut res = Response::new(Body::empty());
                    *res.status_mut() = StatusCode::UNAUTHORIZED;
                    return Ok(res);
                }

                Ok::<_, HyperError>(Response::new(Body::empty()))
            }
        });
        future::ready(Ok::<_, Infallible>(outer))
    });

    // Configure server
    let tcp = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))?;
    let server = Server::from_tcp(tcp)?.http1_only(true).serve(service);

    // Launch Tokio async runtime
    Builder::new_current_thread()
        .enable_io()
        .build()?
        .block_on(server)?;
    Ok(())
}
