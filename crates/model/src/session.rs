use alloc::boxed::Box;
use core::num::NonZeroU64;
use serde::{Deserialize, Serialize};

pub use bson::DateTime;

#[derive(Deserialize, Serialize)]
pub struct Session {
    /// Discord user ID with this session.
    #[serde(rename = "_id")]
    pub user: NonZeroU64,
    /// Access token to the Discord API prefixed by its token type. This is typically set to
    /// `Bearer` For the sake of forward- compatibility, we still include the prefix anyway.
    /// Examples include `Bearer 1234...7890` and `Bearer ABC...DEF`.
    ///
    /// See [Discord's documentation][access] for more details.
    ///
    /// [access]: https://discord.com/developers/docs/topics/oauth2#authorization-code-grant-access-token-response
    pub access: Box<str>,
    /// Refresh token to be used in case we need to access user information again. Note that the
    /// OAuth scope we use only has the `identify` permission.
    ///
    /// [refresh]: https://discord.com/developers/docs/topics/oauth2#authorization-code-grant-refresh-token-exchange-example
    pub refresh: Box<str>,
    /// The specific date at which MongoDB must expire this session.
    pub expires: DateTime,
}
