import { assert } from 'std/testing/asserts.ts';

const GUILD_ID = Deno.env.get('GUILD_ID');
const BOT_TOKEN = Deno.env.get('BOT_TOKEN');
assert(BOT_TOKEN, 'no bot token provided');

export const env = {
    GUILD_ID: GUILD_ID ? BigInt(GUILD_ID) : undefined,
    BOT_TOKEN,
};
