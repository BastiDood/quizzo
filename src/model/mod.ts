import { z } from 'zod';
import { ReadyPayloadDataSchema } from './event.ts';
import { HelloPayloadSchema } from './op.ts';

export { GATEWAY_VERSION, GetGatewayBotSchema } from './gateway.ts';

export const DiscordWebSocketPayloadSchema = z.union([ReadyPayloadDataSchema, HelloPayloadSchema]);
