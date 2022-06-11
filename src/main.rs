use futures_util::{FutureExt, TryFutureExt};
use hyper::{
    header::{HeaderValue, CONTENT_TYPE},
    Body, Response, Server, StatusCode,
};
use lobby::{service, Lobby, APPLICATION_JSON};
use ring::signature::{UnparsedPublicKey, ED25519};
use std::{
    convert::Infallible,
    env, future,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::runtime::Runtime;

fn resolve_json_bytes(bytes: Vec<u8>) -> Response<Body> {
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().append(CONTENT_TYPE, HeaderValue::from_static(APPLICATION_JSON));
    response
}

fn resolve_error_code(code: StatusCode) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = code;
    response
}

fn main() -> anyhow::Result<()> {
    // Retrieve the public key
    let key = env::var("PUB_KEY")?;
    let bytes: Arc<_> = hex::decode(key)?.into();
    let public = UnparsedPublicKey::new(&ED25519, bytes);

    // Parse other environment variables
    let port = env::var("PORT")?.parse()?;
    let app = env::var("APP_ID")?.parse()?;
    let token = env::var("TOKEN")?;

    // Run server
    let lobby = Lobby::new(token, app);
    let addr: SocketAddr = (Ipv4Addr::UNSPECIFIED, port).into();
    Runtime::new()?.block_on(async move {
        use hyper::service::{make_service_fn, service_fn};
        let service = make_service_fn(move |_| {
            let lobby_outer = lobby.clone();
            let public_outer = public.clone();
            future::ready(Ok::<_, Infallible>(service_fn(move |req| {
                let lobby_inner = lobby_outer.clone();
                let public_inner = public_outer.clone();
                service::try_respond(req, lobby_inner, public_inner)
                    .map_ok_or_else(resolve_error_code, resolve_json_bytes)
                    .map(Ok::<_, Infallible>)
            })))
        });
        Server::bind(&addr).http1_only(true).serve(service).await
    })?;
    Ok(())
}
