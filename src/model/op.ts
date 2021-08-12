// deno-lint-ignore-file camelcase
import { z } from 'zod';

const enum OpCode {
    /** An event was dispatched. */
    Dispatch,
    /** Fired periodically by the client to keep the connection alive. */
    Heartbeat,
    /** Starts a new session during the initial handshake. */
    Identify,
    /** Update the client's presence. */
    PresenceUpdate,
    /** Used to join/leave or move between voice channels. */
    VoiceStateUpdate,
    /** Resume a previous session that was disconnected. */
    Resume = 6,
    /** You should attempt to reconnect and resume immediately. */
    Reconnect,
    /** Request information about offline guild members in a large guild. */
    RequestGuildMembers,
    /** The session has been invalidated. You should reconnect and identify/resume accordingly. */
    InvalidSession,
    /** Sent immediately after connecting, contains the `heartbeat_interval` to use. */
    Hello,
    /** Sent in response to receiving a heartbeat to acknowledge that it has been received. */
    HeartbeatAck,
}

interface OpCodePayload<T> {
    op: OpCode;
    d: T;
}

function createOpPayloadSchema<D>(code: OpCode, schema: z.ZodSchema<D>) {
    return z
        .object({
            op: z.literal(code),
            d: schema,
        })
        .strict();
}

interface HelloPayloadData {
    /** The interval (in milliseconds) the client should heartbeat with. */
    heartbeat_interval: number;
}

const HelloPayloadDataSchema: z.ZodSchema<HelloPayloadData> = z.object({
    heartbeat_interval: z.number().positive().int(),
});

export const HelloPayloadSchema: z.ZodSchema<OpCodePayload<HelloPayloadData>> =
    createOpPayloadSchema(OpCode.Hello, HelloPayloadDataSchema);
