import { Discord, Std } from 'deps';
import { incrementWinCount, popQuestion } from 'state';
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

        // Apply initial reactions for UX
        await quiz.addReactions(fields.map(f => f.name), true);

        // Send results of the quiz
        await Std.delay(question.limit);
        await msg.send(`**Time's up! The correct answer is ${fields[question.answer].name}.**`);

        // FIXME: At the moment, we are not detecting whether the user has reacted multiple times.
        // Compute new leaderboard
        const reactions = await Discord.getReactions(quiz, fields[question.answer].name);
        const winners = reactions
            .filter(user => !user.bot && !user.system);
        for (const winner of winners)
            incrementWinCount(winner.id);
    },
};
