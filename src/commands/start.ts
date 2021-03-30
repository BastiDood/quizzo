import { Discord, Std } from 'deps';
import { popQuestion } from '../state.ts';
import type { Command } from './mod.ts';

export const start: Command = {
    help: {
        description: 'Start the current user\'s hosted quiz, if any.',
        usage: '%start',
    },
    async execute(msg, _) {
        const question = popQuestion(msg.author.id);
        if (!question) {
            await msg.reply('You currently don\'t host a quiz! Create one with `%create <url>`.');
            return;
        }

        // Send poll to the text channel
        const fields = question.choices.map((choice, index) => ({
            name: String.fromCodePoint(0x1f1e6 + index),
            value: choice,
        }));
        const quiz = await msg.send({
            embed: {
                color: 0x236EA5,
                author: {
                    name: msg.author.username,
                    icon_url: msg.member?.avatarURL,
                },
                title: 'Quizzo Question',
                description: question.description,
                fields,
                footer: { text: `Time Limit: ${(question.limit / 1e3).toFixed(1)} Seconds` },
            },
        });

        // Apply initial reactions
        await quiz.addReactions(fields.map(f => f.name), true);

        // Wait for a given amount of time and then proceed with the tally
        await Std.delay(question.limit);
        const reactions = Discord.cache.messages.get(quiz.id)!.reactions;
        if (!reactions) {
            await msg.send('No reactions were sent.');
            return;
        }

        // Send results of the quiz
        await msg.send(`**Time's up! The correct answer is ${fields[question.answer].name}.**`);
        // TODO: Compute total reactions and correct answers for the leaderboard feature
    },
};
