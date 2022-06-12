# Quizzo
_Quizzo_ is a [Discord bot](https://discord.com/api/oauth2/authorize?client_id=823813267133956136&scope=applications.commands) for making simple quizzes.

# Development
This bot is powered by the [Twilight library](https://github.com/twilight-rs/twilight) for the [Rust programming language](https://www.rust-lang.org/tools/install). Before running the bot, the following environment variables must be set:

**Variable**    | **Description**
----------------|-----------------------------------------------------------------------------------------------------
`PUB_KEY`       | Hex-encoded cryptograhpic public key provided by the [Discord Developer Portal][discord].
`APP_ID`        | Application ID provided by the [Discord Developer Portal][discord].
`CLIENT_ID`     | Client ID provided by the [Discord Developer Portal][discord].
`CLIENT_SECRET` | Client secret provided by the [Discord Developer Portal][discord].
`REDIRECT_URI`  | Redirect URI to be used when redirecting the user during the [OAuth Authorization Code Flow][oauth].
`TOKEN`         | Bot token provided by the [Discord Developer Portal][discord].
`MONGODB_URI`   | [MongoDB URI connection string][mongodb] that should be used as the database.
`PORT`          | Network port to bind to when launching the bot.

[mongodb]: https://www.mongodb.com/docs/manual/reference/connection-string/
[discord]: https://discord.com/developers/applications
[oauth]: https://discord.com/developers/docs/topics/oauth2#authorization-code-grant

Once these are available, one may use Rust's built-in package manager [Cargo](https://doc.rust-lang.org/cargo/) to launch the bot.

```bash
# Load OAuth environment variables
CLIENT_ID=
CLIENT_SECRET=
REDIRECT_URI=

# Load Discord environment variables
APP_ID=
TOKEN=
PUB_KEY=

# Load network environment variables
PORT=

# Register the required commands
GUILD_ID=
deno run --allow-net --allow-env scripts/register-commands.ts

# Start the bot!
cargo run --release
```
