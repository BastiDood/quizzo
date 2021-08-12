import { env } from './env.ts';
import {
    GATEWAY_VERSION,
    GetGatewayBotSchema,
    DiscordWebSocketPayloadSchema,
} from './model/mod.ts';

// Probe the gateway for the WebSocket URL
const gatewayProbe = await fetch('https://discord.com/api/gateway/bot', {
    method: 'GET',
    headers: { Authorization: env.BOT_TOKEN },
});
const gatewayProbeResponse = await gatewayProbe.json();
const { url: wsUrl } = GetGatewayBotSchema.parse(gatewayProbeResponse);

// Initiate handshake with the gateway
const socket = new WebSocket(wsUrl + `?v=${GATEWAY_VERSION}&encoding=json`);
socket.addEventListener('message', function (evt) {
    const payload = DiscordWebSocketPayloadSchema.parse(JSON.parse(evt.data));
});
