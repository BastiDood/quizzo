use hyper::{
    body::{self, Buf},
    service, Body, Response, Server, Uri,
};
use quizzo::Lobby;
use std::{
    convert::Infallible,
    env, future,
    net::{Ipv4Addr, SocketAddr},
};
use tokio::runtime::Runtime;

fn main() -> anyhow::Result<()> {
    // Parse environment variables
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
        future::ready(Ok::<_, Infallible>(service::service_fn(move |req| {
            let lobby_inner = lobby_outer.clone();
            async move {
                // For now, we only allow requests from the root endpoint.
                anyhow::ensure!(req.uri() == &Uri::from_static("/"));

                // Parse incoming interaction
                let body = req.into_body();
                let reader = body::aggregate(body).await?.reader();
                let interaction = serde_json::from_reader(reader)?;

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
