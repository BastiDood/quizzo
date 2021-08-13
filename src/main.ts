import {
    createSlashCommand,
    sendInteractionResponse,
    startBot,
    DiscordApplicationCommandOptionTypes,
    DiscordMessageComponentTypes,
    DiscordInteractionResponseTypes,
    DiscordInteractionTypes,
} from 'discord';
import { delay } from 'std/async/delay.ts';
import { assert } from 'std/testing/asserts.ts';

import { env } from './env.ts';
import { QuizSchema } from './model/quiz.ts';

const quizzes = new Map<string, Set<string>[]>();

console.log('Starting up the bot...');
await startBot({
    token: env.BOT_TOKEN,
    intents: [],
    compress: true,
    eventHandlers: {
        async ready() {
            console.log('Registering commands...');
            const startCommand = await createSlashCommand(
                {
                    name: 'start',
                    description: 'Start your quiz.',
                    options: [
                        {
                            type: DiscordApplicationCommandOptionTypes.String,
                            name: 'url',
                            description: 'The URL from which to read the JSON quiz.',
                            required: true,
                        },
                    ],
                },
                env.GUILD_ID
            );
            assert(startCommand.name === 'start');

            const startCommandArg = startCommand.options?.at(0);
            assert(startCommandArg?.name === 'url');

            console.log('Bot is ready!');
        },
        async interactionCreate(payload) {
            switch (payload.type) {
                case DiscordInteractionTypes.ApplicationCommand: {
                    // Check command if valid
                    if (payload.data?.name !== 'start') return;

                    // Check argument if there is one
                    const argument = payload.data.options?.at(0);
                    if (argument?.type !== DiscordApplicationCommandOptionTypes.String) return;
                    if (argument.name !== 'url') return;

                    // Check if the user is valid
                    const user = payload.member?.user ?? payload.user;
                    if (!user) return;

                    // Fetch the JSON
                    const response = await fetch(argument.value);
                    const maybeQuiz = QuizSchema.safeParse(await response.json());
                    if (!maybeQuiz.success) {
                        await sendInteractionResponse(payload.id, payload.token, {
                            type: DiscordInteractionResponseTypes.ChannelMessageWithSource,
                            private: true,
                            data: { content: 'Invalid quiz schema.' },
                        });
                        return;
                    }

                    const { timeout, question, answer, choices } = maybeQuiz.data;
                    const customId = crypto.randomUUID();
                    const results = Array.from(choices, _ => new Set<string>());
                    quizzes.set(customId, results);
                    await sendInteractionResponse(payload.id, payload.token, {
                        type: DiscordInteractionResponseTypes.ChannelMessageWithSource,
                        private: false,
                        data: {
                            embeds: [
                                {
                                    type: 'rich',
                                    title: 'Quizzo!',
                                    description: question,
                                    author: {
                                        name: user.username,
                                        iconUrl: `https://cdn.discordapp.com/avatars/${user.id}/${user.avatar}.png`,
                                    },
                                },
                            ],
                            components: [
                                {
                                    type: DiscordMessageComponentTypes.ActionRow,
                                    components: [
                                        {
                                            customId,
                                            type: DiscordMessageComponentTypes.SelectMenu,
                                            placeholder: 'Your Answer',
                                            options: choices.map((choice, index) => ({
                                                value: index.toString(),
                                                label: choice,
                                                default: false,
                                            })),
                                        },
                                    ],
                                },
                            ],
                        },
                    });
                    await delay(timeout * 1000);
                    quizzes.delete(customId);
                    const winners = Array.from(results[answer], id => `<@${id}>`).join(' ');
                    const congrats =
                        winners.length > 0
                            ? `Congratulations to ${winners}!`
                            : 'Nobody got the correct answer...';
                    await sendInteractionResponse(payload.id, payload.token, {
                        type: DiscordInteractionResponseTypes.ChannelMessageWithSource,
                        private: false,
                        data: {
                            content: `The correct answer is ${choices[answer]}! ${congrats}`,
                            allowedMentions: { users: Array.from(results[answer]) },
                        },
                    });
                    break;
                }
                case DiscordInteractionTypes.MessageComponent: {
                    if (payload.data?.componentType !== DiscordMessageComponentTypes.SelectMenu)
                        return;

                    const user = payload.member?.user ?? payload.user;
                    if (!user) return;

                    const quizResult = quizzes.get(payload.data.customId);
                    if (!quizResult) return;

                    const value = payload.data.values.at(0);
                    if (!value) return;

                    // FIXME: Currently, the sets are not mutually exclusive.
                    // Changing one's answer will still count towards their old choice.
                    quizResult.at(Number(value))?.add(user.id);
                    await sendInteractionResponse(payload.id, payload.token, {
                        type: DiscordInteractionResponseTypes.ChannelMessageWithSource,
                        private: true,
                        data: { content: 'Received your answer! Feel free to edit it again!' },
                    });
                    break;
                }
            }
        },
    },
});
