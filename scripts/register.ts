const APP_ID = Deno.env.get('APP_ID');
const TOKEN = Deno.env.get('BOT_TOKEN');
const GUILD_ID = Deno.env.get('GUILD_ID');

// Ensure that `APP_ID` and `TOKEN` are available
if (!APP_ID || !TOKEN)
    throw new Error('missing environment variables');

const endpoint = GUILD_ID
    ? `https://discord.com/api/v10/applications/${APP_ID}/guilds/${GUILD_ID}/commands`
    : `https://discord.com/api/v10/applications/${APP_ID}/commands`;

const qid = {
    type: 4,
    name: 'quiz',
    description: 'The quiz ID.',
    required: true,
    min_value: 1,
    max_value: 32767,
};

const question = {
    type: 3,
    name: 'question',
    description: 'The question being asked.',
    required: true,
};

const choice = {
    type: 3,
    name: 'choice',
    description: 'The new choice to be added.',
    required: true,
};

const index = {
    type: 4,
    name: 'index',
    description: 'The index of the choice to be removed.',
    required: true,
    min_value: 0,
    max_value: 24,
};

const answer = {
    type: 4,
    name: 'answer',
    description: 'Index of the correct answer.',
    required: true,
    min_value: 0,
    max_value: 24,
};

const expiration = {
    type: 4,
    name: 'expiration',
    description: 'How long (in seconds) this quiz can be available once started.',
    required: true,
    min_value: 10,
    max_value: 600,
};

const response = await fetch(endpoint, {
    method: 'PUT',
    headers: {
        Authorization: `Bot ${TOKEN}`,
        'Content-Type': 'application/json',
    },
    body: JSON.stringify([
        {
            name: 'create',
            description: 'Create a new quiz with default options.',
            options: [question],
        },
        {
            name: 'list',
            description: 'List down all the quizzes you created.',
        },
        {
            name: 'start',
            description: 'Start a previously created quiz. This deletes it from your list of quizzes.',
            options: [qid],
        },
        {
            name: 'add',
            description: 'Add a new choice to the quiz.',
            options: [qid, choice],
        },
        {
            name: 'remove',
            description: 'Remove a choice from the quiz.',
            options: [qid, index],
        },
        {
            name: 'edit',
            description: 'Edit a property of the quiz.',
            options: [
                {
                    type: 1,
                    name: 'question',
                    description: 'Edit the question itself.',
                    options: [qid, question],
                },
                {
                    type: 1,
                    name: 'answer',
                    description: 'Edit the correct answer of the quiz.',
                    options: [qid, answer],
                },
                {
                    type: 1,
                    name: 'expiration',
                    description: 'Edit the expiration time of the quiz.',
                    options: [qid, expiration],
                },
            ],
        },
        {
            name: 'help',
            description: 'Summon a help menu. Will be sent to you via a temporary message.',
        },
        {
            name: 'about',
            description: 'Some information about the bot, its development, and the creator.',
        },
    ]),
});

console.log(await response.json());
