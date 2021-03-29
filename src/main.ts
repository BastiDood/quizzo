import { Discord, Dotenv } from 'deps';
import { COMMANDS } from './commands/mod.ts';

const { BOT_TOKEN } = Dotenv.config({ export: false, safe: true });

Discord.startBot({
    token: BOT_TOKEN,
    compress: true,
    intents: [ 'GUILDS', 'GUILD_MESSAGES', 'GUILD_MESSAGE_REACTIONS' ],
    eventHandlers: {
        async messageCreate(message) {
            // Ignore system and bot messages as well
            // as the non-prefixed ones
            if (!message.content.startsWith('%') || message.author.bot || message.author.system || !message.channel)
                return;

            // Parse text command
            const [ cmd, ...args ] = message.content
                .slice(1)
                .trimRight()
                .replaceAll(/\s+/g, ' ')
                .split(' ');

            await COMMANDS.getCommand(cmd)?.execute(message, args);
        },
        ready() { console.log('Bot is online!'); },
    },
});
