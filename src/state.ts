import { Discord } from 'deps';

export class Question {
    /** The actual description of the question. */
    readonly description: string;
    /** Array of choices. */
    readonly choices: string[];
    /** Index of the correct answer with respect to the `choices`. */
    readonly answer: number;
    /** Time limit of the question. */
    readonly limit: number;

    /**
     * # Criteria for a Valid Question
     * 1. All the required fields are defined.
     * 2. The length of the `choices` array is `2..=5`.
     * 3. The `answer` is a valid index in the `choices` array.
     * 4. The time `limit` is within `5_000..=60_000` milliseconds.
     *
     * If any of these conditions are not met,
     * then this indirect constructor returns `null`.
     */
    static create(desc: string, choices: string[], answer: number, limit: number): Question | null {
        const { length } = choices;
        return (2 <= length && length <= 5
            && 0 <= answer && answer < length
            && 5_000 <= limit && limit <= 60_000)
            ? new Question(desc, choices, answer, limit)
            : null;
    }

    private constructor(desc: string, choices: string[], answer: number, limit: number) {
        this.description = desc;
        this.choices = choices;
        this.answer = answer;
        this.limit = limit;
    }
}

/**
 * This registry maps a **message ID** to
 * the currently active question session.
 */
const quizzes = new Map<Discord.Message, Question>();

/** Player leaderboard. */
const leaderboard = new Map<string, number>();

export function getQuestionFromMessage(msg: Discord.Message): Question | undefined {
    return quizzes.get(msg);
}

export function setQuestionFromMessage(msg: Discord.Message, question: Question) {
    quizzes.set(msg, question);
}

/** Increments the current user's win count. */
export function incrementWinCount(userID: string) {
    const count = leaderboard.get(userID) ?? 0;
    leaderboard.set(userID, count + 1);
}

/**
 * Returns a sorted array of tuples
 * representing the current leaderboard.
 */
export function getLeaderboard(): { name: string, count: number }[] {
    const { members } = Discord.cache;
    return Array.from(leaderboard.entries(), ([ id, count ]) => {
        const { username, discriminator } = members.get(id)!;
        return {
            name: `${username}#${discriminator}`,
            count,
        };
    }).sort((a, b) => {
        const diff = b.count - a.count;
        return diff !== 0 ? diff : a.name.localeCompare(b.name);
    });
}
