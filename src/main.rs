use hyper::{Body, Response, StatusCode};

fn resolve_error_code(code: StatusCode) -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = code;
    response
}

fn main() -> anyhow::Result<()> {
    use std::{
        env,
        net::{Ipv4Addr, SocketAddr},
    };
    use tokio::runtime::Runtime;

    // Retrieve the public key
    let key = env::var("PUB_KEY")?;
    let pub_key: Box<_> = hex::decode(key)?.into();

    // Parse other environment variables
    let port = env::var("PORT")?.parse()?;
    let app_id = env::var("APP_ID")?.parse()?;
    let client_id = env::var("CLIENT_ID")?;
    let client_secret = env::var("CLIENT_SECRET")?;
    let redirect_uri = env::var("REDIRECT_URI")?;
    let token = env::var("TOKEN")?;
    let mongo = env::var("MONGODB_URI")?;

    // Run server
    use rand_chacha::rand_core::SeedableRng;
    let rng = rand_chacha::ChaChaRng::from_entropy();
    let addr: SocketAddr = (Ipv4Addr::UNSPECIFIED, port).into();
    Runtime::new()?.block_on(async move {
        use api::{App, MongoClient};
        use hyper::Server;
        use std::{convert::Infallible, future, sync::Arc};

        let client = MongoClient::with_uri_str(mongo).await?;
        let db = client.database("quizzo");
        let app = Arc::new(App::new(rng, &db, token, app_id, pub_key, &client_id, &client_secret, &redirect_uri));
        drop(client);

        use hyper::service::{make_service_fn, service_fn};
        let service = make_service_fn(move |_| {
            let app_outer = app.clone();
            future::ready(Ok::<_, Infallible>(service_fn(move |req| {
                let app_inner = app_outer.clone();
                async move { Ok::<_, Infallible>(app_inner.try_respond(req).await.unwrap_or_else(resolve_error_code)) }
            })))
        });

        Server::bind(&addr).http1_only(true).serve(service).await?;
        anyhow::Ok(())
    })?;
    Ok(())
}
