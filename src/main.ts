import { Discord, Dotenv } from 'deps';
import { _clearAll, _clearAllByEmojiName, _receiveReaction, _removeReaction } from './collector.ts';
import { getCommand } from './commands/mod.ts';

const { BOT_TOKEN } = Dotenv.config({ export: false, safe: true });

Discord.startBot({
    token: BOT_TOKEN,
    compress: true,
    intents: [ 'Guilds', 'GuildMembers', 'GuildMessageReactions' ],
    eventHandlers: {
        async messageCreate(message) {
            // Ignore system and bot messages as well
            // as the non-prefixed ones
            if (!message.content.startsWith('%') || message.isBot)
                return;

            // Parse text command
            const [ cmd, ...args ] = message.content
                .slice(1)
                .trimRight()
                .replaceAll(/\s+/g, ' ')
                .split(' ');

            await getCommand(cmd)?.execute(message, args);
        },
        reactionAdd: _receiveReaction,
        reactionRemove: _removeReaction,
        reactionRemoveAll: _clearAll,
        reactionRemoveEmoji: _clearAllByEmojiName,
        ready() { console.log('Bot is online!'); },
    },
});
