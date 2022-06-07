use alloc::{boxed::Box, string::String, vec::Vec};
use core::num::NonZeroU64;
use hyper::{header::HeaderValue, Response, StatusCode, Uri};
use mongodb::{
    bson::{oid::ObjectId, Binary},
    results::InsertOneResult,
    Collection,
};
use rand_core::{CryptoRng, RngCore};

pub async fn create_session<Rand>(
    rng: &mut Rand,
    col: &Collection<Result<NonZeroU64, Binary>>,
) -> mongodb::error::Result<(ObjectId, Box<str>)>
where
    Rand: RngCore + CryptoRng,
{
    use mongodb::bson::spec::BinarySubtype;
    let mut nonce = Vec::from([0; 16]);
    rng.fill_bytes(nonce.as_mut_slice());
    let bin = Binary {
        bytes: nonce,
        subtype: BinarySubtype::Generic,
    };

    let mut txt = Vec::from([0; 32]);
    hex::encode_to_slice(&bin.bytes, &mut txt).unwrap();
    let encoded = String::from_utf8(txt).unwrap().into_boxed_str();

    let InsertOneResult { inserted_id, .. } = col.insert_one(&Err(bin), None).await?;
    let oid = inserted_id.as_object_id().unwrap();
    Ok((oid, encoded))
}

pub fn create_redirect_response(client_id: &str, redirect_uri: &Uri) -> impl Fn(&str, &str) -> Response<()> {
    let form = alloc::format!(
        "https://discord.com/api/oauth2/authorize?response_type=code&client_id={client_id}&redirect_uri={redirect_uri}&state="
    );
    move |session, nonce| {
        let uri = form.clone() + nonce;
        let cookie = alloc::format!("sid={session}; HttpOnly; SameSite=Lax; Secure");

        let (mut parts, body) = Response::new(()).into_parts();
        parts.status = StatusCode::FOUND;
        parts
            .headers
            .insert("Set-Cookie", HeaderValue::from_str(&cookie).unwrap());
        parts.headers.insert("Location", HeaderValue::from_str(&uri).unwrap());
        Response::from_parts(parts, body)
    }
}
