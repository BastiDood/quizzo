import { Discord } from 'deps';

// Command Implementations
import { leaderboard } from './leaderboard.ts';
import { ping } from './ping.ts';
import { start } from './start.ts';

export interface Command {
    readonly help: {
        readonly description: string;
        readonly usage: string;
    };
    execute(msg: Discord.DiscordenoMessage, args: string[]): Promise<void>;
}

/** Command registry. */
const commands = new Map<string, Command>([
    [ 'leaderboard', leaderboard ],
    [ 'ping', ping ],
    [ 'start', start ],
]);

/** Queries for the given command name. */
export function getCommand(key: string) {
    if (key !== 'help')
        return commands.get(key);

    const fields = Array.from(commands.values(), ({ help }) => ({ name: `\`${help.usage}\``, value: help.description }));
    const execute = async (msg: Discord.DiscordenoMessage, _: string[]) => {
        await msg.send({
            embed: {
                title: 'Quizzo Help',
                color: 0x236EA5,
                fields,
            },
        });
    };
    return {
        execute,
        help: {
            description: 'Get help information.',
            usage: '%help',
        },
    } as Command;
}
