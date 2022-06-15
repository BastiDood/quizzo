use super::Lobby;
use db::Database;
use hyper::{Body, HeaderMap, Response, StatusCode};
use ring::signature::UnparsedPublicKey;

pub async fn try_respond<Bytes>(
    body: Body,
    headers: &HeaderMap,
    db: &Database,
    public: &UnparsedPublicKey<Bytes>,
    lobby: &Lobby,
) -> Result<Response<Body>, StatusCode>
where
    Bytes: AsRef<[u8]>,
{
    // Retrieve security headers
    let maybe_sig = headers.get("X-Signature-Ed25519");
    let maybe_time = headers.get("X-Signature-Timestamp");
    let (sig, timestamp) = maybe_sig.zip(maybe_time).ok_or(StatusCode::UNAUTHORIZED)?;
    let signature = hex::decode(sig).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Append body after the timestamp
    let payload = hyper::body::to_bytes(body).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut message = timestamp.as_bytes().to_vec();
    message.extend_from_slice(&payload);

    // Validate the challenge
    public.verify(&message, &signature).map_err(|_| StatusCode::UNAUTHORIZED)?;
    drop(message);
    drop(signature);

    // Parse incoming interaction
    let interaction = serde_json::from_slice(&payload).map_err(|_| StatusCode::BAD_REQUEST)?;
    drop(payload);

    // Construct new body
    let reply = lobby.on_interaction(db, interaction).await;
    let bytes = serde_json::to_vec(&reply).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    use hyper::header::{HeaderValue, CONTENT_TYPE};
    let mut res = Response::new(Body::from(bytes));
    assert!(res.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/json")).is_none());
    Ok(res)
}
