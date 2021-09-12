use hyper::{
    service::{make_service_fn, service_fn},
    Body, Response, Server,
};
use quizzo::{model::Interaction, validate_request, AppError};
use ring::signature::{UnparsedPublicKey, ED25519};
use std::{
    convert::Infallible,
    env, future,
    net::{Ipv4Addr, TcpListener},
    num::NonZeroU64,
    sync::Arc,
};
use tokio::runtime::Builder;

fn main() -> Result<(), AppError> {
    // Try to parse public key
    let public_key = env::var("PUBLIC_KEY")?;
    let pub_bytes: Arc<[u8]> = hex::decode(public_key).map_err(|_| AppError::MalformedEnvVars)?.into();
    let pub_key = UnparsedPublicKey::new(&ED25519, pub_bytes);

    // Retrieve other environment variables
    let port = env::var("PORT")?.parse().map_err(|_| AppError::MalformedEnvVars)?;
    let application_id = env::var("APPLICATION_ID")?;
    let guild_id = env::var("GUILD_ID")?.parse::<u64>().ok().and_then(NonZeroU64::new);

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
                let interaction: Interaction = serde_json::from_slice(&body).unwrap();
                Ok::<_, Infallible>(Response::new(Body::empty()))
            }
        });
        future::ready(Ok::<_, Infallible>(outer))
    });

    // Configure server
    let tcp = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))?;
    let server = Server::from_tcp(tcp)?.http1_only(true).serve(service);

    // Launch Tokio async runtime
    Builder::new_current_thread().enable_io().build()?.block_on(server)?;
    Ok(())
}
