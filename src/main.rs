use hyper::{service, Server};
use quizzo::Lobby;
use std::{
    env,
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

    // Create service handler
    let service = service::make_service_fn(move |_| async move {
        let lobby = Lobby::new(token, app, maybe_guild_id).await?;
        todo!()
    });

    // Run the server
    let addr: SocketAddr = (Ipv4Addr::UNSPECIFIED, port).into();
    let server = Server::bind(&addr).serve(service);
    let res = Runtime::new()?.block_on(server)?;
    anyhow::Ok(res)
}
