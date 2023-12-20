fn main() -> anyhow::Result<()> {
    env_logger::init();
    log::info!("starting up");

    // Retrieve the public key
    use std::env::var;
    let pub_key = var("PUB_KEY")?.into_bytes();
    let mut pub_bytes = [0; 32];
    hex::decode_to_slice(pub_key, &mut pub_bytes)?;
    let pub_key = api::VerifyingKey::from_bytes(&pub_bytes)?;
    log::debug!("loaded public key");

    // Set up Postgres driver configuration
    let app_port = var("PORT")?.parse()?;
    let app_id = var("APP_ID")?.parse()?;
    let bot_token = var("BOT_TOKEN")?;
    let config = var("PG_URL")?.parse::<api::Config>()?;

    use std::net::{Ipv4Addr, TcpListener};
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, app_port))?;
    listener.set_nonblocking(true)?;

    let addr = listener.local_addr()?;
    log::info!("listening to {addr}");

    let runtime = tokio::runtime::Builder::new_multi_thread().enable_io().enable_time().build()?;
    let tcp = {
        let _guard = runtime.enter();
        tokio::net::TcpListener::from_std(listener)?
    };

    runtime.block_on(async {
        let (client, connection) = loop {
            // HACK: Railway Private Networking requires 100ms to set up.
            let err = match config.connect(api::NoTls).await {
                Ok(pair) => break pair,
                Err(err) => err,
            };
            log::error!("{err}");
            tokio::time::sleep(core::time::Duration::from_secs(1)).await;
        };

        use core::pin::pin;
        let mut postgres = pin!(runtime.spawn(connection));
        log::info!("PostgreSQL driver connected");

        let app = api::App::new(client.into(), app_id, bot_token, pub_key);
        let state = std::sync::Arc::new(app);

        let http = hyper::server::conn::http1::Builder::new();
        let mut stop = pin!(tokio::signal::ctrl_c());
        loop {
            tokio::select! {
                Ok((stream, _)) = tcp.accept() => {
                    let outer = state.clone();
                    let service = hyper::service::service_fn(move |req| {
                        let inner = outer.clone();
                        let (hyper::http::request::Parts { method, uri, headers, .. }, body) = req.into_parts();
                        async move {
                            let mut response = Default::default();
                            inner.try_respond(&mut response, method, uri.path(), headers, body).await;
                            Ok::<_, core::convert::Infallible>(response)
                        }
                    });
                    let io = hyper_util::rt::TokioIo::new(stream);
                    runtime.spawn(http.serve_connection(io, service));
                    continue;
                }
                stop_res = &mut stop => {
                    log::info!("stop signal received");
                    stop_res?;
                    break;
                },
                conn_res = &mut postgres => {
                    log::info!("PostgreSQL disconnected");
                    conn_res??;
                    break;
                },
                else => continue,
            }
        }
        anyhow::Ok(())
    })?;

    log::info!("shutting down");
    Ok(())
}
