import { Discord } from 'deps';

/** Maps a **user ID** to a set of messages. */
type UserReactions = Map<string, Set<string>>;

/**
 * This reaction collector locally accumulates the reactions
 * from messages sent via the `reaction-*` events. It is essentially
 * a mapping from a **message ID** to some mapping of a **user ID**
 * and their respective reactions.
 */
const messages = new Map<string, UserReactions>();

/**
 * Begin collecting messages for the given **message ID**.
 * Note that this overwrites previous accumulations, if any.
 */
export function beginCollectingFor(id: string) {
    messages.set(id, new Map());
}

/** Removes the **message ID** from listeners. */
export function finishCollectingFor(id: string): UserReactions | undefined {
    const collector = messages.get(id);
    messages.delete(id);
    return collector;
}

// deno-lint-ignore camelcase
export function _receiveReaction({ id, user_id, emoji, member }: Discord.MessageReactionUncachedPayload) {
    if (emoji.name === null || member?.user.bot || member?.user.system)
        return;

    const msg = messages.get(id);
    if (msg === undefined)
        return;

    const reactions = msg.get(user_id);
    if (reactions)
        reactions.add(emoji.name)
    else
        msg.set(user_id, new Set([ emoji.name ]));
}

export function _removeReaction({ id, emoji, member }: Discord.MessageReactionUncachedPayload, userID: string) {
    if (emoji.name === null || member?.user.bot || member?.user.system)
        return;

    const msg = messages.get(id);
    if (msg === undefined)
        return;

    const reactions = msg.get(userID);
    if (reactions === undefined)
        return;

    // Also remove user from accumulation
    // if they have no reactions left
    reactions.delete(emoji.name);
    if (reactions.size < 1)
        msg.delete(userID);
}

export function _clearAll(id: string) {
    messages.delete(id);
}

export function _clearAllByName(name: string) {
    for (const reactions of messages.values())
        for (const [ userID, reax ] of reactions) {
            reax.delete(name);
            if (reax.size < 1)
                reactions.delete(userID);
        }
}
