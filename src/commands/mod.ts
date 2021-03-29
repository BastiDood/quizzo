import { Discord } from 'deps';
import { create } from './create.ts';
import { ping } from './ping.ts';

export interface Command {
    help: {
        description: string;
        usage: string;
    };
    execute(msg: Discord.Message, args: string[]): Promise<void>;
}

/** Command registry. */
const commands = new Map<string, Command>([
    [ 'ping', ping ],
    [ 'create', create ],
]);

/** Queries for the given command name. */
export function getCommand(key: string) {
    if (key === 'help')
        throw new Error('not yet implemented');
    return commands.get(key);
}
