# Quizzo
_Quizzo_ is a [Discord bot](https://discord.com/api/oauth2/authorize?client_id=823813267133956136&scope=applications.commands) for making simple quizzes.

# Development
This bot is powered by the [Twilight library](https://github.com/twilight-rs/twilight) for the [Rust programming language](https://www.rust-lang.org/tools/install). Before running the bot, the following environment variables must be set:

**Variable**  | **Description**                                                                           | Required? | Default
------------- | ----------------------------------------------------------------------------------------- | :-------: | ------:
`PORT`        | Network port to bind to when launching the bot.                                           | &#x2714   |
`PUB_KEY`     | Hex-encoded cryptograhpic public key provided by the [Discord Developer Portal][discord]. | &#x2714   |
`BOT_TOKEN`   | Bot token provided by the [Discord Developer Portal][discord].                            | &#x2714   |
`PG_PORT`     | Port at which the PostgreSQL instance is hosted.                                          | &#x274c   | `5432`
`PG_HOSTNAME` | Hostname at which the PostgreSQL instance is hosted.                                      | &#x2714   |
`PG_DATABASE` | Default database to which the PostgreSQL driver must connect to.                          | &#x2714   |
`PG_USERNAME` | Username for PostgreSQL user authentication.                                              | &#x2714   |
`PG_PASSWORD` | Password for PostgreSQL user authentication.                                              | &#x2714   |

[discord]: https://discord.com/developers/applications

Once these are available, one may use Rust's built-in package manager [Cargo](https://doc.rust-lang.org/cargo/) to launch the bot.

```bash
# Register the required commands
BOT_TOKEN=
GUILD_ID=
deno run --allow-net --allow-env scripts/register-commands.ts

# Start the bot!
PORT=
PUB_KEY=
PG_PORT=5432
cargo run --release
```
