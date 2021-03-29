interface Question {
    description: string;
    choices: string[];
}

/** Quiz registry. */
const quizzes = new Map<string, Question>();

/** Sets the current quiz of the given user. */
export function setQuestion(userID: string, description: string, choices: string[]) {
    quizzes.set(userID, { description, choices });
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
