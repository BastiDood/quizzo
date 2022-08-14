use hyper::{Body, Response, StatusCode};

fn resolve_error_code(code: StatusCode) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = code;
    response
}

fn main() -> anyhow::Result<()> {
    // Retrieve the public key
    use std::env::var;
    let key = var("PUB_KEY")?;
    let pub_key: Box<_> = hex::decode(key)?.into();

    // Parse other environment variables
    let port = var("PORT")?.parse()?;
    let app_id = var("APP_ID")?.parse()?;
    let client_id = var("CLIENT_ID")?;
    let client_secret = var("CLIENT_SECRET")?;
    let redirect_uri = var("REDIRECT_URI")?;
    let token = var("TOKEN")?;
    let mongo = var("MONGODB_URI")?;

    // Set up runtime and TCP listener
    use std::net::{Ipv4Addr, TcpListener};
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))?;
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_io().enable_time().build()?;
    let tcp = {
        let _guard = runtime.enter();
        tokio::net::TcpListener::from_std(listener)?
    };

    let rng: rand_chacha::ChaChaRng = rand_chacha::rand_core::SeedableRng::from_entropy();
    runtime.block_on(async {
        let client = api::MongoClient::with_uri_str(mongo).await.expect("cannot connect to Mongo");
        let db = client.database("quizzo");
        drop(client);

        let app = api::App::new(rng, &db, token, app_id, pub_key, &client_id, &client_secret, &redirect_uri);
        let state = std::sync::Arc::new(app);

        let mut http = hyper::server::conn::Http::new();
        http.http1_only(true);

        let stop = tokio::signal::ctrl_c();
        tokio::pin!(stop);
        let signal = loop {
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
                stop_res = &mut stop => break stop_res,
                else => continue,
            }
        };

        signal.expect("cannot process termination signal");
    });

    Ok(())
}
