import { Discord, Dotenv } from 'deps';
import { COMMANDS } from './commands/mod.ts';

const { BOT_TOKEN } = Dotenv.config({ export: false, safe: true });

Discord.startBot({
    token: BOT_TOKEN,
    compress: true,
    intents: [
        'DIRECT_MESSAGES',
        'DIRECT_MESSAGE_REACTIONS',
        'GUILD_MESSAGES',
        'GUILD_MESSAGE_REACTIONS',
    ],
    eventHandlers: {
        async messageCreate(message) {
            // Ignore system and bot messages as well
            // as the non-prefixed ones
            if (message.author.bot || message.author.system || !message.content.startsWith('%'))
                return;

            // Parse text command
            const [ cmd, ...args ] = message.content
                .slice(1)
                .trimRight()
                .replaceAll(/\s+/g, ' ')
                .split(' ');

            switch (message.channel?.type) {
                // Respond to guild messages
                case 0:
                    await COMMANDS.getGlobalCommand(cmd)?.execute(message, args);
                    break;
                // Respond to DMs
                case 1:
                    await COMMANDS.getCommand(cmd)?.execute(message, args);
                    break;
            }
        },
        ready() { console.log('Bot is online!'); },
    },
});
