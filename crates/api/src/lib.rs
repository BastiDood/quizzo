#![no_std]

extern crate alloc;

mod auth;
mod quiz;

use alloc::string::String;
use auth::{CodeExchanger, Redirect};
use db::Database;

pub use db::{MongoClient, MongoDb};
pub use hyper::Uri;

pub struct App {
    /// Handle to the database collections.
    db: Database,
    /// Wrapper for the Discord API bot.
    discord: twilight_http::Client,
    /// Redirects requests to the OAuth consent page.
    redirector: Redirect,
    /// Exchanges authorization codes for token responses.
    exchanger: CodeExchanger,
    /// HTTPS/1.0 client for token-related API calls.
    http: hyper::Client<hyper_trust_dns::RustlsHttpsConnector>,
}

impl App {
    pub fn new(db: &MongoDb, bot_token: String, client_id: &str, client_secret: &str, redirect_uri: &Uri) -> Self {
        let connector = hyper_trust_dns::TrustDnsResolver::default().into_rustls_native_https_connector();
        let http = hyper::Client::builder()
            .http1_max_buf_size(8192)
            .set_host(false)
            .build(connector);
        Self {
            db: Database::new(db),
            discord: twilight_http::Client::new(bot_token),
            exchanger: CodeExchanger::new(client_id, client_secret, redirect_uri),
            redirector: Redirect::new(client_id, redirect_uri),
            http,
        }
    }
}
