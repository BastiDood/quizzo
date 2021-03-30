import { Discord } from 'deps';

/** Maps a **user ID** to a set of messages. */
type UserReactions = Map<string, Set<string>>;

/**
 * This reaction collector locally accumulates user message
 * messages sent from the `reaction-*` events.
 */
const messages = new Map<string, UserReactions>();

/**
 * Begin collecting messages for the given **message ID**.
 * Note that this overwrites previous collections, if any.
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
