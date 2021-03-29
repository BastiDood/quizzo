import { setQuestion } from '../state.ts';
import type { Command } from './mod.ts';

export const create: Command = {
    help: {
        description: 'Set the given URL as the current user\'s active quiz.',
        usage: '%create url',
    },
    async execute(msg, args) {
        try {
            const url = new URL(args[0]);
            const response = await fetch(url);
            const text = await response.text();

            // Parsing the text file requires the text into lines.
            // The first line serves as the question itself, while
            // the remaining lines are the choices. Note that the
            // choices must be length 2..=10 to be valid.
            //
            // # Example
            // What does comes after 'A' in the alphabet?
            // B
            // C
            // D
            // E
            const [ question, ...choices ] = text.split(/[\r\n]+/);
            setQuestion(msg.author.id, question, choices);
            await msg.reply('Successfully set you as the host of this quiz!')
        } catch {
            await msg.reply('Could not parse questionnaire.');
        }
    },
};
