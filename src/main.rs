use hyper::{body, service, Body, Response, Server, StatusCode, Uri};
use quizzo::Lobby;
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
    let maybe_guild_id = match env::var("GUILD_ID") {
        Ok(guild_id) => Some(guild_id.parse()?),
        _ => None,
    };

    // Initialize service handler
    let runtime = Runtime::new()?;
    let future = Lobby::new(token, app, maybe_guild_id);
    let lobby = runtime.block_on(future)?;
    let service = service::make_service_fn(move |_| {
        let lobby_outer = lobby.clone();
        let public_outer = public.clone();
        future::ready(Ok::<_, Infallible>(service::service_fn(move |req| {
            let lobby_inner = lobby_outer.clone();
            let public_inner = public_outer.clone();
            async move {
                // For now, we only allow requests from the root endpoint.
                if req.uri() != &Uri::from_static("/") {
                    let mut response = Response::new(Body::empty());
                    *response.status_mut() = StatusCode::NOT_FOUND;
                    return Ok(response);
                }

                // Retrieve security headers
                let headers = req.headers();
                let maybe_sig = headers.get("X-Signature-Ed25519").and_then(|val| val.to_str().ok());
                let maybe_time = headers.get("X-Signature-Timestamp").and_then(|val| val.to_str().ok());
                let (sig, timestamp) = if let Some(pair) = maybe_sig.zip(maybe_time) {
                    pair
                } else {
                    let mut response = Response::new(Body::empty());
                    *response.status_mut() = StatusCode::BAD_REQUEST;
                    return Ok(response);
                };

                // Verify security headers
                let signature = hex::decode(sig)?;
                let mut message = timestamp.as_bytes().to_vec();
                let bytes = body::to_bytes(req.into_body()).await?;
                message.extend_from_slice(&bytes);
                if public_inner.verify(&message, &signature).is_err() {
                    let mut response = Response::new(Body::empty());
                    *response.status_mut() = StatusCode::UNAUTHORIZED;
                    return Ok(response);
                }

                // Parse incoming interaction
                drop(signature);
                drop(message);
                let interaction = serde_json::from_slice(&bytes)?;

                // Reply to the server
                let response = lobby_inner.on_interaction(interaction).await;
                let body: Body = serde_json::to_vec(&response)?.into();
                anyhow::Ok(Response::new(body))
            }
        })))
    });

    // Run the server
    let addr: SocketAddr = (Ipv4Addr::UNSPECIFIED, port).into();
    let server = Server::bind(&addr);
    runtime.block_on(server.serve(service))?;
    Ok(())
}
