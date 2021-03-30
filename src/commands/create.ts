import { Question, setQuestion } from '../state.ts';
import type { Command } from './mod.ts';

function validateQuestion(json: unknown): json is Question {
    if (typeof json !== 'object' || json === null)
        return false;

    const { description, answer, choices, limit } = json as Partial<Question>;
    return typeof description === 'string'
        && typeof answer === 'number'
        && choices instanceof Array
        && choices.every(item => typeof item === 'string')
        && typeof limit === 'number';
}

export const create: Command = {
    help: {
        description: 'Set the given URL as the current user\'s active quiz. The URL should link to a valid JSON file.',
        usage: '%create <url>',
    },
    async execute(msg, args) {
        try {
            const url = new URL(args[0]);
            const headers = new Headers({ 'Accept': 'application/json' })
            const response = await fetch(url, {
                method: 'GET',
                headers,
            });
            const json: unknown = await response.json();

            if (!validateQuestion(json))
                throw new TypeError('invalid JSON input');

            if (!setQuestion(msg.author.id, json))
                throw new TypeError('invalid question parameters');

            await msg.reply('Successfully set you as the host of this quiz!');
        } catch {
            await msg.reply('Could not parse questionnaire.');
        }
    },
};
