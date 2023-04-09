use hyper::{Body, Response, StatusCode};

fn resolve_error_code(code: StatusCode) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = code;
    response
}

fn main() -> anyhow::Result<()> {
    // Retrieve the public key
    use std::env::{var, VarError};
    let key = var("PUB_KEY")?;
    let pub_key = hex::decode(key)?.into_boxed_slice();

    // Set up Postgres driver configuration
    let app_port = var("PORT")?.parse()?;
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

    // Set up runtime
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_io().enable_time().build()?;
    let tcp = {
        let _guard = runtime.enter();
        tokio::net::TcpListener::from_std(listener)?
    };

    env_logger::init();
    runtime.block_on(async {
        use core::pin::pin;
        let (client, connection) = config.connect(api::NoTls).await?;
        let mut postgres = pin!(runtime.spawn(connection));

        let app = api::App::new(client.into(), bot_token, pub_key);
        let state = std::sync::Arc::new(app);

        let mut http = hyper::server::conn::Http::new();
        http.http1_only(true);

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
                    let future = http.serve_connection(stream, service);
                    runtime.spawn(async { future.await.unwrap() });
                    continue;
                }
                stop_res = &mut stop => {
                    stop_res?;
                    break;
                },
                conn_res = &mut postgres => {
                    conn_res??;
                    break;
                },
                else => continue,
            }
        }
        anyhow::Ok(())
    })
}
