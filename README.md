# Quizzo
_Quizzo_ is a [Discord bot](https://discord.com/api/oauth2/authorize?client_id=823813267133956136&scope=applications.commands) for making simple quizzes.

# Development
This bot is powered by the [Serenity framework](https://docs.rs/serenity) for the [Rust programming language](https://www.rust-lang.org/tools/install). Before running the bot, the following environment variables must be set:

**Variable**     | **Description**
---------------- | -------------------------------------------------------------------------------------------------------
`GUILD_ID`       | Guild to which commands are locally set.
`APPLICATION_ID` | Application ID provided by the [Discord Developer Portal](https://discord.com/developers/applications).
`BOT_TOKEN`      | Bot token provided by the [Discord Developer Portal](https://discord.com/developers/applications).

Once these are available, one may use Rust's built-in package manager [Cargo](https://doc.rust-lang.org/cargo/) to launch the bot.

```bash
# Start the bot!
cargo run --release
```
