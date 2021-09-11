use hyper::{
    body::Bytes,
    body::{to_bytes, HttpBody},
    Method, Request, StatusCode,
};
use ring::signature::UnparsedPublicKey;

pub async fn validate_request<P, B>(
    req: Request<B>,
    pub_key: &UnparsedPublicKey<P>,
) -> Result<Bytes, StatusCode>
where
    P: AsRef<[u8]>,
    B: HttpBody,
{
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
    let body = to_bytes(req.into_body())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    message.extend_from_slice(&body);
    pub_key
        .verify(&message, &signature)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    Ok(body)
}
