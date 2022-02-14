use hyper::{
    header::{HeaderValue, CONTENT_TYPE},
    Body, Response, Server,
};
use quizzo::{lobby::Lobby, service};
use ring::signature::{UnparsedPublicKey, ED25519};
use std::{
    convert::Infallible,
    env, future,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::runtime::Runtime;

fn main() -> anyhow::Result<()> {
    // Retrieve the public key
    let key = env::var("PUB_KEY")?;
    let bytes: Arc<_> = hex::decode(key)?.into();
    let public = UnparsedPublicKey::new(&ED25519, bytes);

    // Parse other environment variables
    let port = env::var("PORT")?.parse()?;
    let app = env::var("APP_ID")?.parse()?;
    let token = env::var("TOKEN")?;

    // Prepare server
    let lobby = Lobby::new(token, app);
    let service = hyper::service::make_service_fn(move |_| {
        let lobby_outer = lobby.clone();
        let public_outer = public.clone();
        future::ready(Ok::<_, Infallible>(hyper::service::service_fn(move |req| {
            let lobby_inner = lobby_outer.clone();
            let public_inner = public_outer.clone();
            async move {
                let future = service::try_respond(req, &lobby_inner, &public_inner);
                let response = match future.await {
                    Ok(bytes) => {
                        let mut response = Response::new(Body::from(bytes));
                        response
                            .headers_mut()
                            .append(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                        response
                    }
                    Err(code) => {
                        let mut response = Response::new(Body::empty());
                        *response.status_mut() = code;
                        response
                    }
                };
                Ok::<_, Infallible>(response)
            }
        })))
    });

    // Run server
    let addr: SocketAddr = (Ipv4Addr::UNSPECIFIED, port).into();
    Runtime::new()?.block_on(Server::bind(&addr).serve(service))?;
    Ok(())
}
