use hyper::{
    body::{self, Bytes, HttpBody},
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, StatusCode, {Error as HyperError, Server},
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
    MalformedEnvVars,
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

async fn validate_request<T: AsRef<[u8]>, B: HttpBody>(
    req: Request<B>,
    pub_key: &UnparsedPublicKey<T>,
) -> Result<Bytes, StatusCode> {
    // Disallow non-POST methods and unexpected paths
    if req.method() != Method::POST || req.uri().path() != "/" {
        return Err(StatusCode::NOT_FOUND);
    }

    // Check existence of signatures
    let signature = req
        .headers()
        .get("X-Signature-Ed25519")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut message = req
        .headers()
        .get("X-Signature-Timestamp")
        .ok_or(StatusCode::BAD_REQUEST)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .as_bytes()
        .to_vec();
    if signature.is_empty() || message.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Verify signatures
    let signature = hex::decode(signature).map_err(|_| StatusCode::BAD_REQUEST)?;
    let body = body::to_bytes(req.into_body())
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    message.extend_from_slice(&body);
    pub_key
        .verify(&message, &signature)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    Ok(body)
}

fn main() -> Result<(), AppError> {
    // Try to parse public key
    let public_key = env::var("PUBLIC_KEY")?;
    let pub_bytes: Arc<[u8]> = hex::decode(public_key)
        .map_err(|_| AppError::MalformedEnvVars)?
        .into();
    let pub_key = UnparsedPublicKey::new(&ED25519, pub_bytes);

    // Retrieve other environment variables
    let port = env::var("PORT")?
        .parse()
        .map_err(|_| AppError::MalformedEnvVars)?;
    let application_id = env::var("APPLICATION_ID")?;
    let guild_id = env::var("GUILD_ID")?
        .parse::<u64>()
        .ok()
        .and_then(NonZeroU64::new);

    // Configure main service
    let service = make_service_fn(move |_| {
        let outer_pub_key = pub_key.clone();
        let outer = service_fn(move |req| {
            let inner_pub_key = outer_pub_key.clone();
            async move {
                let body = match validate_request(req, &inner_pub_key).await {
                    Ok(body) => body,
                    Err(code) => {
                        let mut res = Response::<Body>::default();
                        *res.status_mut() = code;
                        return Ok(res);
                    }
                };
                Ok::<_, Infallible>(Response::new(Body::empty()))
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
