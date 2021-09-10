use ed25519_dalek::{PublicKey, Signature, Verifier};
use hyper::{
    body,
    service::{make_service_fn, service_fn},
    Body, Method, Response, StatusCode, {Error as HyperError, Server},
};
use std::{
    convert::{Infallible, TryInto},
    env::{self, VarError},
    future,
    io::Error as IoError,
    net::{Ipv4Addr, TcpListener},
    num::NonZeroU64,
};
use tokio::runtime::Builder;

#[derive(Debug)]
enum AppError {
    MissingEnvVars,
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

    // Cache the public key
    let pub_bytes = hex::decode(application_id).map_err(|_| AppError::MissingEnvVars)?;
    let pub_key = PublicKey::from_bytes(&pub_bytes).map_err(|_| AppError::MissingEnvVars)?;

    // Configure main service
    let service = make_service_fn(move |_| {
        let outer = service_fn(move |req| {
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
                let sig = hex::decode(signature).ok();
                let ts = hex::decode(timestamp).ok();
                let (signature, mut timestamp) = match sig.zip(ts) {
                    Some(pair) => pair,
                    _ => {
                        let mut res = Response::new(Body::empty());
                        *res.status_mut() = StatusCode::BAD_REQUEST;
                        return Ok(res);
                    }
                };

                // Verify signatures
                let signature: Signature = signature.as_slice().try_into().unwrap();
                let body = body::to_bytes(req.into_body()).await?;
                timestamp.extend_from_slice(&body);
                pub_key.verify(&timestamp, &signature).unwrap();

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
