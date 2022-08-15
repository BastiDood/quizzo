use alloc::boxed::Box;
use core::num::NonZeroU64;
use serde::{Deserialize, Serialize};

pub use bson::DateTime;

#[derive(Deserialize, Serialize)]
pub enum Session {
    /// A session currently in the consent page. Once the callback is triggered, the nonce will be
    /// used to verify the query parameters. This should mitigate most instances of [Cross-Site
    /// Request Forgery][csrf].
    ///
    /// [csrf]: https://docs.microsoft.com/en-us/aspnet/web-api/overview/security/preventing-cross-site-request-forgery-csrf-attacks
    Pending {
        /// One-time salt to be used for hashing the session.
        nonce: u64,
    },
    /// At this point, the OAuth callback parameters have been validated.
    Valid {
        /// Discord ID of the user with this session.
        user: NonZeroU64,
        /// Access token to the Discord API prefixed by its token type. This is typically set to
        /// `Bearer` For the sake of forward- compatibility, we still include the prefix anyway.
        /// Examples include `Bearer 1234...7890` and `Bearer ABC...DEF`.
        ///
        /// See [Discord's documentation][access] for more details.
        ///
        /// [access]: https://discord.com/developers/docs/topics/oauth2#authorization-code-grant-access-token-response
        access: Box<str>,
        /// Refresh token to be used in case we need to access user information again. Note that the
        /// OAuth scope we use only has the `identify` permission.
        ///
        /// [refresh]: https://discord.com/developers/docs/topics/oauth2#authorization-code-grant-refresh-token-exchange-example
        refresh: Box<str>,
        /// The specific date at which MongoDB must expire this session.
        expires: DateTime,
    },
}

impl Session {
    pub const fn as_user(&self) -> Option<NonZeroU64> {
        if let Self::Valid { user, .. } = *self {
            Some(user)
        } else {
            None
        }
    }

    pub const fn as_nonce(&self) -> Option<u64> {
        if let Self::Pending { nonce } = *self {
            Some(nonce)
        } else {
            None
        }
    }
}
