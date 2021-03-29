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
    #globalCommands = new Map<string, Command>([
        [ 'ping', ping ]
    ]);

    /** These are commands that are only available via DMs. */
    #directCommands = new Map<string, Command>([]);

    /** Queries for a given command available globally. */
    getGlobalCommand(key: string) {
        if (key === 'help')
            throw new Error('not yet implemented');
        return this.#globalCommands.get(key);
    }

    /**
     * Queries for the appropriate global command first.
     * Otherwise, it then searches for DM-specific commands.
     */
    getCommand(key: string) { return this.getGlobalCommand(key) ?? this.#directCommands.get(key) }
}

export const COMMANDS = new CommandManager();
