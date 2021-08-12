# Quizzo
_Quizzo_ is a [Discord bot](https://discord.com/api/oauth2/authorize?client_id=823813267133956136&permissions=75840&scope=bot) for making simple quizzes.

# Development
This bot is powered by the [Deno](https://deno.land/) TypeScript runtime. To start the bot, provide a `.env` file in the project root directory containing your application's `BOT_TOKEN`. Alternatively, this can be defined as an environment variable beforehand.

The `BOT_TOKEN` can be obtained from the [Discord Developer Application Console](https://discord.com/developers/applications). See [`.env.example`](.env.example) for example.

Once the `.env` file is available in the root directory, it is now possible to run the bot with the following command:

```bash
deno run --config=tsconfig.json --import-map=imports.json --allow-env --allow-net --allow-env src/main.ts
```
