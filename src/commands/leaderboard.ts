import { getLeaderboard } from 'state';
import type { Command } from './mod.ts';

export const leaderboard: Command = {
    help: {
        description: 'Display the current leaderboard.',
        usage: '%leaderboard',
    },
    async execute(msg, _) {
        const leaderboard = getLeaderboard();
        if (leaderboard.length < 1) {
            await msg.send('The leaderboards are currently empty!');
            return;
        }

        const text = leaderboard.map(([ name, count ], index) => `${index + 1}. ${name} [${count}]`).join('\n');
        await msg.send(`**Leaderboard:**\n${text}`);
    },
};
