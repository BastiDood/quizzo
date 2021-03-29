# Quizzo
_Quizzo_ is a Discord bot for making simple quizzes.

# Development
This bot is powered by the [Deno](https://deno.land/) TypeScript runtime. To start the bot, provide a `.env` file in the project root directory containing your application's `BOT_TOKEN`. This can be obtained from the [Discord Developer Application Console](https://discord.com/developers/applications). See the [`.env.example`](.env.example) for example.

Once the `.env` file is available in the root directory, it is now possible to run the bot with the following command:

```bash
deno run --import-map=imports.json --allow-read --allow-net --allow-env src/main.ts
```
