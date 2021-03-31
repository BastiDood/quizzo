import { beginCollectingFor, finishCollectingFor } from 'collector';
import { Discord, Std, Zod } from 'deps';
import { Question, incrementWinCount } from 'state';
import type { Command } from './mod.ts';

const QuestionSchema = Zod.object({
    description: Zod.string(),
    choices: Zod.string().array(),
    answer: Zod.number(),
    limit: Zod.number(),
});
const URLChecker = Zod.string().url();

export const start: Command = {
    help: {
        description: 'Start a quiz based on the JSON information from a URL.',
        usage: '%start <url>',
    },
    async execute(msg, args) {
        // Check if URL is valid
        const urlResult = URLChecker.safeParse(args[0]);
        if (!urlResult.success) {
            await msg.send('Invalid URL.');
            return;
        }

        // Fetch the data as JSON
        const url = new URL(urlResult.data);
        const headers = new Headers({ 'Accept': 'application/json' });
        const response = await fetch(url, {
            method: 'GET',
            headers,
        });

        // Validate JSON object
        const questionResult = QuestionSchema.safeParse(await response.json());
        if (!questionResult.success) {
            await msg.send('Invalid JSON object.');
            return;
        }

        // Validate question parameters
        const { data: { description, choices, answer, limit } } = questionResult;
        const question = Question.create(description, choices, answer, limit);
        if (question === null) {
            await msg.send('Invalid question parameters.');
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

        // Begin quiz timer once all reaction options are sent
        beginCollectingFor(quiz.id);
        await quiz.addReactions(fields.map(f => f.name), true);
        await Std.delay(question.limit);
        const collector = finishCollectingFor(quiz.id)!;

        // Find the winners
        const correctAnswer = fields[question.answer].name;
        const winners = Array.from(collector.entries())
            .filter(([ userID, reactions ]) => {
                // Remove bot reactions
                const user = Discord.cache.members.get(userID);
                if (user?.bot || user?.system)
                    return false;

                // Only check the first emoji reaction
                const isCorrect = reactions.values().next().value === correctAnswer;
                if (isCorrect)
                    incrementWinCount(userID);
                return isCorrect;
            })
            .map(([ userID, _ ]) => `<@${userID}>`);

        // Congratulate the winners
        const mentions = winners.length > 0 ? ` Congratulations ${winners.join(' ')}!` : ' Nobody got it right...';
        await msg.send(`**Time's up! The correct answer is ${correctAnswer}.**${mentions}`);
    },
};
