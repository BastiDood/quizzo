import { Discord } from 'deps';

/** Maps a **user ID** to a set of reactions. */
type UserReactions = Map<string, Set<string>>;

/**
 * This reaction collector locally accumulates the reactions
 * from messages sent via the `reaction-*` events. It is essentially
 * a mapping from a **message ID** to some mapping of a **user ID**
 * and their respective reactions.
 */
const messages = new Map<bigint, UserReactions>();

/**
 * Begin collecting messages for the given **message ID**.
 * Note that this overwrites previous accumulations, if any.
 */
export function beginCollectingFor(id: bigint) {
    messages.set(id, new Map());
}

/** Removes the **message ID** from listeners. */
export function finishCollectingFor(id: bigint): UserReactions | undefined {
    const collector = messages.get(id);
    messages.delete(id);
    return collector;
}

export function _receiveReaction(data: Discord.MessageReactionAdd, msg?: Discord.DiscordenoMessage) {
    const emoji = data.emoji.name;
    if (msg === undefined || msg.isBot || emoji === null || emoji === undefined)
        return;

    const userReactions = messages.get(msg.id);
    if (userReactions === undefined)
        return;

    const reactions = userReactions.get(data.userId);
    if (reactions)
        reactions.add(emoji)
    else
        userReactions.set(data.userId, new Set([ emoji ]));
}

export function _removeReaction(data: Discord.MessageReactionRemove, msg?: Discord.DiscordenoMessage) {
    if (msg === undefined || msg.isBot || data.emoji.name === null || data.emoji.name === undefined)
        return;

    const users = messages.get(msg.id);
    if (users === undefined)
        return;

    const reactions = users.get(data.userId);
    if (reactions === undefined)
        return;

    // Also remove user from accumulation
    // if they have no reactions left
    reactions.delete(data.emoji.name);
    if (reactions.size < 1)
        users.delete(data.userId);
}

export function _clearAll(_: Discord.MessageReactionRemoveAll, msg?: Discord.DiscordenoMessage) {
    if (msg === undefined)
        return;
    messages.delete(msg.id);
}

export function _clearAllByEmojiName(emoji: Partial<Discord.Emoji>, msgId: bigint) {
    const { name } = emoji;
    if (name === undefined || name === null)
        return;

    const userReactions = messages.get(msgId);
    if (userReactions === undefined)
        return;

    for (const [ userId, reactions ] of userReactions.entries()) {
        reactions.delete(name);
        if (reactions.size < 1)
            userReactions.delete(userId);
    }
}
