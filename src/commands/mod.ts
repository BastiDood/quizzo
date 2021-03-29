import { Discord } from 'deps';
import { ping } from './ping.ts';

export interface Command {
    help: {
        description: string;
        usage: string;
    },
    execute(msg: Discord.Message, args: string[]): Promise<void>;
}

class CommandManager {
    /** These are commands that are available in the global context. */
    #commands = new Map<string, Command>([
        [ 'ping', ping ]
    ]);

    /** Queries for the given command name. */
    getCommand(key: string) {
        if (key === 'help')
            throw new Error('not yet implemented');
        return this.#commands.get(key);
    }
}

export const COMMANDS = new CommandManager();
