# Quizzo
_Quizzo_ is a [Discord bot](https://discord.com/api/oauth2/authorize?client_id=823813267133956136&scope=applications.commands) for making simple quizzes.

# Development
This bot is powered by the [Twilight library](https://github.com/twilight-rs/twilight) for the [Rust programming language](https://www.rust-lang.org/tools/install). Before running the bot, the following environment variables must be set:

**Variable** | **Description**
-------------|---------------------------------------------------------------------
`PUB_KEY`    | Cryptograhpic public key provided by the [Discord Developer Portal].
`APP_ID`     | Application ID provided by the [Discord Developer Portal].
`TOKEN`      | Bot token provided by the [Discord Developer Portal].
`PORT`       | Network port to bind to when launching the bot.

[Discord Developer Portal]: https://discord.com/developers/applications

Once these are available, one may use Rust's built-in package manager [Cargo](https://doc.rust-lang.org/cargo/) to launch the bot.

```bash
# Load Required Environment Variables
PUB_KEY=
APP_ID=
TOKEN=
PORT=
GUILD_ID=

# Register the required commands
deno run --allow-net --allow-env scripts/register-commands.ts

# Start the bot!
cargo run --release
```
