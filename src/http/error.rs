use hyper::http::{self, uri};
use std::io;

pub enum FetchError {
    Hyper(hyper::Error),
    Http(hyper::http::Error),
    Io(io::Error),
    Uri(uri::InvalidUri),
}

impl From<http::Error> for FetchError {
    fn from(err: http::Error) -> Self {
        Self::Http(err)
    }
}

impl From<hyper::Error> for FetchError {
    fn from(err: hyper::Error) -> Self {
        Self::Hyper(err)
    }
}

impl From<serde_json::Error> for FetchError {
    fn from(err: serde_json::Error) -> Self {
        Self::Io(err.into())
    }
}

impl From<io::Error> for FetchError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<uri::InvalidUri> for FetchError {
    fn from(err: uri::InvalidUri) -> Self {
        Self::Uri(err)
    }
}
