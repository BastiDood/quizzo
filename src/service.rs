use super::lobby::{Lobby, APPLICATION_JSON};
use hyper::{
    body::{self, HttpBody},
    header::{HeaderValue, CONTENT_TYPE},
    Body, Method, Request, Response, StatusCode, Uri,
};
use ring::signature::UnparsedPublicKey;
use std::sync::Arc;

type ArcSlice = Arc<[u8]>;
type PublicKey = UnparsedPublicKey<ArcSlice>;
pub async fn try_respond<B: HttpBody>(
    req: Request<B>,
    lobby: Lobby,
    public: PublicKey,
) -> Result<Vec<u8>, StatusCode> {
    // Disable all non-`POST` requests
    if req.method() != Method::POST {
        return Err(StatusCode::METHOD_NOT_ALLOWED);
    }

    // For now, we only allow requests from the root endpoint.
    if req.uri() != &Uri::from_static("/") {
        return Err(StatusCode::NOT_FOUND);
    }

    // Retrieve security headers
    let headers = req.headers();
    let maybe_sig = headers.get("X-Signature-Ed25519").and_then(|val| val.to_str().ok());
    let maybe_time = headers.get("X-Signature-Timestamp").and_then(|val| val.to_str().ok());
    let (sig, timestamp) = maybe_sig.zip(maybe_time).ok_or(StatusCode::BAD_REQUEST)?;
    let signature = hex::decode(sig).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Verify security headers
    let mut message = timestamp.as_bytes().to_vec();
    let bytes = body::to_bytes(req.into_body())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    message.extend_from_slice(&bytes);
    public
        .verify(&message, &signature)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    drop(message);
    drop(signature);
    drop(public);

    // Parse incoming interaction
    let interaction = serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    drop(bytes);

    // Construct new body
    let reply = lobby.on_interaction(interaction).await;
    let bytes = serde_json::to_vec(&reply).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(bytes)
}

pub fn resolve_json_bytes(bytes: Vec<u8>) -> Response<Body> {
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .append(CONTENT_TYPE, HeaderValue::from_static(APPLICATION_JSON));
    response
}

pub fn resolve_error_code(code: StatusCode) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = code;
    response
}
