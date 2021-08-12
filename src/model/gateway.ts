// deno-lint-ignore-file camelcase
import { z } from 'zod';

export const GATEWAY_VERSION = 9;

interface SessionStartLimit {
    /** The total number of session starts the current user is allowed. */
    total: number;
    /** The remaining number of session starts the current user is allowed. */
    remaining: number;
    /** The number of milliseconds after which the limit resets. */
    reset_after: number;
    /** The number of identify requests allowed per five seconds. */
    max_concurrency: number;
}

const SessionStartLimitSchema: z.ZodSchema<SessionStartLimit> = z
    .object({
        total: z.number().int(),
        remaining: z.number().int(),
        reset_after: z.number().int(),
        max_concurrency: z.number().int(),
    })
    .strict();

/** Gateway information. */
interface GetGatewayBot {
    /** The WSS URL that can be used for connecting to the gateway. */
    url: string;
    /** The recommended number of shards to use when connecting. */
    shards: number;
    /** Information on the current session start limit. */
    session_start_limit: SessionStartLimit;
}

export const GetGatewayBotSchema: z.ZodSchema<GetGatewayBot> = z
    .object({
        url: z.string().url(),
        shards: z.number().int(),
        session_start_limit: SessionStartLimitSchema,
    })
    .strict();
