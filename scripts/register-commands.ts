const APP_ID = Deno.env.get('APP_ID');
const TOKEN = Deno.env.get('BOT_TOKEN');
const GUILD_ID = Deno.env.get('GUILD_ID');

// Ensure that `APP_ID` and `TOKEN` are available
if (!APP_ID || !TOKEN)
    throw new Error('missing environment variables');

const endpoint = GUILD_ID
    ? `https://discord.com/api/v10/applications/${APP_ID}/guilds/${GUILD_ID}/commands`
    : `https://discord.com/api/v10/applications/${APP_ID}/commands`;

const response = await fetch(endpoint, {
    method: 'PUT',
    headers: {
        Authorization: `Bot ${TOKEN}`,
        'Content-Type': 'application/json',
    },
    body: JSON.stringify([
        {
            name: 'start',
            description: 'Start your most recently submitted quiz.',
            options: [
                {
                    type: 3,
                    name: 'url',
                    description: 'URL from which to retrieve the JSON data.',
                    required: true,
                    min_value: 1,
                    max_value: 1,
                }
            ],
        },
        {
            name: 'help',
            description: 'Summon a help menu. Will be sent to you via a temporary message.',
        },
    ]),
});

const json = await response.json();
console.log(json);
