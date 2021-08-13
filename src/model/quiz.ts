import { z } from 'zod';

export interface Quiz {
    /** Number of seconds that this question is valid. */
    timeout: number;
    /** The question prompt itself. */
    question: string;
    /** Possible options. */
    choices: string[];
    /** Must be within the range of `options`. */
    answer: number;
}

export const QuizSchema: z.ZodSchema<Quiz> = z
    .object({
        timeout: z.number().positive().int().min(15).max(30),
        question: z.string().nonempty(),
        choices: z.string().nonempty().array(),
        answer: z.number().nonnegative().int(),
    })
    .superRefine((val, ctx) => {
        const { length } = val.choices;
        if (val.answer >= length)
            ctx.addIssue({
                code: 'too_big',
                type: 'number',
                inclusive: false,
                maximum: length,
            });
    });
