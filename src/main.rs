use hyper::{
    service::{make_service_fn, service_fn},
    Body, Response, {Error as HyperError, Server},
};
use std::{
    convert::Infallible,
    env::{self, VarError},
    future,
    io::Error as IoError,
    net::{Ipv4Addr, TcpListener},
    num::NonZeroU64,
};
use tokio::runtime::Builder;

#[derive(Debug)]
enum AppError {
    ServerInit,
    MissingEnvVars,
    Hyper(HyperError),
    Io(IoError),
}

impl From<VarError> for AppError {
    fn from(_: VarError) -> Self {
        Self::MissingEnvVars
    }
}

impl From<IoError> for AppError {
    fn from(err: IoError) -> Self {
        Self::Io(err)
    }
}

impl From<HyperError> for AppError {
    fn from(err: HyperError) -> Self {
        Self::Hyper(err)
    }
}

fn main() -> Result<(), AppError> {
    // Retrieve environment variables
    let port = env::var("PORT")?
        .parse()
        .map_err(|_| AppError::MissingEnvVars)?;
    let bot_token = env::var("BOT_TOKEN")?;
    let application_id = env::var("APPLICATION_ID")?
        .parse::<u64>()
        .map_err(|_| AppError::MissingEnvVars)?;
    let guild_id = env::var("GUILD_ID")?
        .parse::<u64>()
        .ok()
        .and_then(NonZeroU64::new);

    // Configure main service
    let service = make_service_fn(|_| {
        let outer =
            service_fn(|_| future::ready(Ok::<_, Infallible>(Response::new(Body::empty()))));
        future::ready(Ok::<_, Infallible>(outer))
    });

    // Configure server
    let tcp = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))?;
    let server = Server::from_tcp(tcp)?.http1_only(true).serve(service);

    // Launch Tokio async runtime
    Builder::new_current_thread()
        .enable_io()
        .build()?
        .block_on(server)?;
    Ok(())
}
