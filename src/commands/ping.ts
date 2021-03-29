import type { Command } from './mod.ts';

export const ping: Command = {
    help: {
        description: 'Pong!',
        usage: '%ping',
    },
    async execute(msg, _) {
        await msg.reply('Pong!');
    },
};
