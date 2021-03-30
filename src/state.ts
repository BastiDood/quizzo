/**
 * Criteria for a valid question:
 * 1. All the required fields are defined.
 * 2. The length of the `choices` array is `2..=10`.
 * 3. The `answer` is a valid index in the `choices` array.
 * 4. The time `limit` is within `5_000..=60_000` milliseconds.
 */
export interface Question {
    /** The actual description of the question. */
    description: string;
    choices: string[];
    /** Index of the correct answer. */
    answer: number;
    /** Time limit in milliseconds. */
    limit: number;
}

/** Quiz registry. */
const quizzes = new Map<string, Question>();

/**
 * Sets the current quiz of the given user.
 * This returns `true` if the given question
 * is valid. See the documentation for the
 * [`Question`] interface for more information.
 */
export function setQuestion(userID: string, q: Question): boolean {
    const { length } = q.choices;
    if (length < 2 || length > 10)
        return false;

    const { answer } = q
    if (answer < 0 || answer >= length)
        return false;

    const { limit } = q;
    if (limit < 5_000 || limit > 60_000)
        return false;

    quizzes.set(userID, q);
    return true;
}

/**
 * Removes the user's currently set question for the user.
 * This question is then returned to the caller, if applicable.
 */
export function popQuestion(userID: string): Question | undefined {
    const question = quizzes.get(userID);
    if (!question)
        return;

    quizzes.delete(userID);
    return question;
}
