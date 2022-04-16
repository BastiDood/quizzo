use hyper::{http::uri::Scheme, Uri};

/// Validates whether the URI is "trusted". As long as the URI follows the
/// format `https://cdn.discordapp.com/{snowflake}/{snowflake}/{filename}.json`,
/// the validation should pass.
pub fn is_allowed_uri(uri: &Uri) -> bool {
    if uri.scheme() != Some(&Scheme::HTTPS) {
        return false;
    }

    if uri.host() != Some("cdn.discordapp.com") {
        return false;
    }

    let rest = match uri.path().strip_prefix("/attachments/") {
        Some(suffix) => suffix,
        _ => return false,
    };

    let mut comps = rest.split('/');
    if !comps
        .by_ref()
        .take(2)
        .all(|comp| comp.as_bytes().iter().all(u8::is_ascii_digit))
    {
        return false;
    }

    if let Some(name) = comps.next() {
        return name.ends_with(".json");
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unencrypted_http() {
        assert!(!is_allowed_uri(&Uri::from_static("http://localhost")));
        assert!(!is_allowed_uri(&Uri::from_static("http://cdn.discordapp.com")));
        assert!(!is_allowed_uri(&Uri::from_static("http://example.com")));
        assert!(!is_allowed_uri(&Uri::from_static("http://google.com")));
        assert!(!is_allowed_uri(&Uri::from_static("http://hello.world")));
    }

    #[test]
    fn rejects_wrong_host() {
        assert!(!is_allowed_uri(&Uri::from_static("https://localhost")));
        assert!(!is_allowed_uri(&Uri::from_static("https://example.com")));
        assert!(!is_allowed_uri(&Uri::from_static("https://google.com")));
        assert!(!is_allowed_uri(&Uri::from_static("https://hello.world")));
        assert!(!is_allowed_uri(&Uri::from_static("https://evil.net")));
    }

    #[test]
    fn rejects_incorrect_cdn_endpoint() {
        assert!(!is_allowed_uri(&Uri::from_static("https://cdn.discordapp.com/api")));
        assert!(!is_allowed_uri(&Uri::from_static("https://cdn.discordapp.com/evil")));
        assert!(!is_allowed_uri(&Uri::from_static("https://cdn.discordapp.com/hello")));
        assert!(!is_allowed_uri(&Uri::from_static("https://cdn.discordapp.com/world")));
        assert!(!is_allowed_uri(&Uri::from_static("https://cdn.discordapp.com/example")));
    }

    #[test]
    fn rejects_invalid_snowflakes() {
        assert!(!is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments"
        )));
        assert!(!is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/abc"
        )));
        assert!(!is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/abc/def"
        )));
        assert!(!is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/123/abc"
        )));
        assert!(!is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/abc/123"
        )));
    }

    #[test]
    fn rejects_invalid_file_extensions() {
        assert!(!is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/123/456/Question.exe"
        )));
        assert!(!is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/123/456/Question.so"
        )));
        assert!(!is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/123/456/Question.sh"
        )));
        assert!(!is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/123/456/Question.txt"
        )));
        assert!(!is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/123/456/Question.md"
        )));
    }

    #[test]
    fn accepts_valid_formats() {
        assert!(is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/123/456/Question.json"
        )));
        assert!(is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/456/789/Template.json"
        )));
        assert!(is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/12203934/90823432/Hello.json"
        )));
        assert!(is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/12203934/90823432/World.json"
        )));
        assert!(is_allowed_uri(&Uri::from_static(
            "https://cdn.discordapp.com/attachments/12203934/90823432/Example.json"
        )));
    }
}
