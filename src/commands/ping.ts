import type { Command } from './mod.ts';

export const ping: Command = {
    help: {
        description: 'Pong!',
        usage: '%pong',
    },
    async execute(msg, _) {
        await msg.reply('Pong!');
    },
};
