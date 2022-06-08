use alloc::{
    boxed::Box,
    fmt::{self, Formatter},
};
use core::num::NonZeroU64;
use serde::{
    de::{MapAccess, Visitor},
    Deserialize, Deserializer,
};

pub struct TokenResponse {
    /// Accessed token prefixed with the token type (typically `Bearer`).
    pub access: Box<str>,
    /// Refresh token.
    pub refresh: Box<str>,
    /// Number of seconds until expiration.
    pub expires: NonZeroU64,
}

struct TokenVisitor;

#[derive(Deserialize)]
enum StrOrNum<'txt> {
    Str(&'txt str),
    Num(NonZeroU64),
}

impl<'de> Visitor<'de> for TokenVisitor {
    type Value = TokenResponse;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a valid token response")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        use serde::de::{Error, Unexpected};

        let mut access = None::<Box<str>>;
        let mut refresh = None::<Box<str>>;
        let mut expires = None::<NonZeroU64>;
        let mut bearer = true;

        while let Some(pair) = map.next_entry::<&str, StrOrNum>()? {
            match pair {
                ("access_token", _) if access.is_some() => return Err(A::Error::duplicate_field("access_token")),
                ("refresh_token", _) if refresh.is_some() => return Err(A::Error::duplicate_field("refresh_token")),
                ("expires_in", _) if expires.is_some() => return Err(A::Error::duplicate_field("expires_in")),
                ("token_type", _) if bearer => return Err(A::Error::duplicate_field("token_type")),
                ("access_token" | "refresh_token" | "token_type", StrOrNum::Num(num)) => {
                    let unexp = Unexpected::Unsigned(num.get());
                    return Err(A::Error::invalid_type(unexp, &"text"));
                }
                ("expires_in", StrOrNum::Str(val)) => {
                    let unexp = Unexpected::Str(val);
                    return Err(A::Error::invalid_type(unexp, &"number"));
                }
                ("token_type", StrOrNum::Str("Bearer")) => bearer = true,
                ("token_type", StrOrNum::Str(val)) => {
                    let unexp = Unexpected::Str(val);
                    return Err(A::Error::invalid_value(unexp, &"Bearer"));
                }
                ("access_token", StrOrNum::Str(token)) => {
                    let text = alloc::format!("Bearer {token}");
                    access = Some(text.into_boxed_str());
                }
                ("refresh_token", StrOrNum::Str(token)) => refresh = Some(token.into()),
                ("expires_in", StrOrNum::Num(num)) => expires = Some(num),
                (field, _) => {
                    const EXPECTED_FIELDS: [&str; 4] = ["access_token", "refresh_token", "expires_in", "token_type"];
                    return Err(A::Error::unknown_field(field, &EXPECTED_FIELDS));
                }
            }
        }

        if !bearer {
            return Err(A::Error::missing_field("token_type"));
        }

        Ok(Self::Value {
            access: access.ok_or_else(|| A::Error::missing_field("access_token"))?,
            refresh: refresh.ok_or_else(|| A::Error::missing_field("refresh_token"))?,
            expires: expires.ok_or_else(|| A::Error::missing_field("expires_in"))?,
        })
    }
}

impl<'de> Deserialize<'de> for TokenResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(TokenVisitor)
    }
}
