CREATE TABLE quiz(
    -- Monotonically increasing ID for each quiz.
    id SMALLSERIAL NOT NULL,
    -- Discord User ID.
    author BIGINT NOT NULL CHECK(author != 0),
    -- The actual question being asked.
    question VARCHAR(100) NOT NULL CHECK(question != ''),
    -- Possible choices to the question.
    choices VARCHAR(100)[]
        NOT NULL
        DEFAULT '{}'
        CONSTRAINT quiz_choices_length_check
        CHECK(ARRAY_LENGTH(choices, 1) <= 25),
    -- Index of the answer into the array.
    answer SMALLINT
        CHECK(answer IS NULL OR 0 <= answer AND answer < ARRAY_LENGTH(choices, 1)),
    -- Number of seconds before the quiz expires.
    expiration SMALLINT NOT NULL DEFAULT 10 CHECK(timeout BETWEEN 10 AND 600),
    PRIMARY KEY (id, author)
);
