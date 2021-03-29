import { Discord, Dotenv } from 'deps';

const { BOT_TOKEN } = Dotenv.config({ export: false, safe: true });

Discord.startBot({
    token: BOT_TOKEN,
    compress: true,
    intents: [ 'GUILDS', 'GUILD_MESSAGES' ],
    eventHandlers: {
        async messageCreate(message) {
            if (message.channel?.type !== 0 || message.author.bot || message.author.system)
                return;

            if (message.content == 'ping')
                await message.channel?.send('pong');
        },
    },
});
