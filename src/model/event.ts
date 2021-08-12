// deno-lint-ignore-file camelcase
import { z } from 'zod';
import { GATEWAY_VERSION } from './gateway.ts';

interface ApplicationInformation {
    id: string;
    name: string;
    description: string;
}

const ApplicationInformationSchema: z.ZodSchema<ApplicationInformation> = z.object({
    id: z.string().nonempty(),
    name: z.string().nonempty(),
    description: z.string(),
});

interface UnavailableGuild {
    id: string;
    unavailable: false;
}

const UnavailableGuildSchema: z.ZodSchema<UnavailableGuild> = z
    .object({
        id: z.string().nonempty(),
        unavailable: z.literal(false),
    })
    .strict();

interface ReadyPayloadData {
    v: typeof GATEWAY_VERSION;
    /** Used for resuming connections. */
    session_id: string;
    guilds: UnavailableGuild[];
    application: ApplicationInformation;
}

export const ReadyPayloadDataSchema: z.ZodSchema<ReadyPayloadData> = z.object({
    v: z.literal(GATEWAY_VERSION),
    session_id: z.string().nonempty(),
    guilds: UnavailableGuildSchema.array(),
    application: ApplicationInformationSchema,
});
