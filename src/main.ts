import { env } from './env.ts';
import { startBot, DiscordInteractionTypes } from 'discord';

await startBot({
    token: env.BOT_TOKEN,
    intents: [],
    compress: true,
    eventHandlers: {
        ready() {
            console.log('Bot is ready!');
        },
        interactionCreate(payload) {
            switch (payload.type) {
                case DiscordInteractionTypes.ApplicationCommand:
                    return;
                case DiscordInteractionTypes.MessageComponent:
                    return;
            }
        },
    },
});
