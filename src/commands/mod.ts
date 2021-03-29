import { Discord } from 'deps';
import { ping } from './ping.ts';

export interface Command {
    help: {
        description: string;
        usage: string;
    };
    execute(msg: Discord.Message, args: string[]): Promise<void>;
}

const commands = new Map<string, Command>([
    [ 'ping', ping ]
]);

/** Queries for the given command name. */
export function getCommand(key: string) {
    if (key === 'help')
        throw new Error('not yet implemented');
    return commands.get(key);
}
