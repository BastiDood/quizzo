use http_body_util::Full;
use hyper::{Response, StatusCode, body::Bytes};

fn resolve_error_code(code: StatusCode) -> Response<Full<Bytes>> {
    let mut response = Response::default();
    *response.status_mut() = code;
    response
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    log::info!("Starting up");

    // Retrieve the public key
    use std::env::{var, VarError};
    let key = var("PUB_KEY")?;
    let pub_key = hex::decode(key)?.into_boxed_slice();

    // Set up Postgres driver configuration
    let app_port = var("PORT")?.parse()?;
    let app_id = var("APP_ID")?.parse()?;
    let bot_token = var("BOT_TOKEN")?;
    let config = {
        let username = var("PG_USERNAME")?;
        let password = var("PG_PASSWORD")?;
        let hostname = var("PG_HOSTNAME")?;
        let database = var("PG_DATABASE")?;
        let db_port = match var("PG_PORT") {
            Ok(port) => port.parse()?,
            Err(VarError::NotPresent) => 5432,
            Err(err) => return Err(anyhow::Error::new(err)),
        };
        let mut config = api::Config::new();
        config.user(&username).password(&password).host(&hostname).port(db_port).dbname(&database);
        config
    };

    // Set up TCP listener
    use std::net::{Ipv4Addr, TcpListener};
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, app_port))?;
    listener.set_nonblocking(true)?;

    let addr = listener.local_addr()?;
    log::info!("Listening to {addr}");

    // Set up runtime
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_io().enable_time().build()?;
    let tcp = {
        let _guard = runtime.enter();
        tokio::net::TcpListener::from_std(listener)?
    };

    runtime.block_on(async {
        use core::pin::pin;
        let (client, connection) = config.connect(api::NoTls).await?;
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
                        async move {
                            let response = inner.try_respond(req).await.unwrap_or_else(resolve_error_code);
                            Ok::<_, core::convert::Infallible>(response)
                        }
                    });
                    let io = hyper_util::rt::TokioIo::new(stream);
                    runtime.spawn(http.serve_connection(io, service));
                    continue;
                }
                stop_res = &mut stop => {
                    log::info!("Stop signal received");
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

    log::info!("Shutting down");
    Ok(())
}
