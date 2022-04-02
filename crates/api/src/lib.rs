use http::StatusCode;
use model::Quiz;
use mongodb::{results::InsertOneResult, Collection};

/// Attempts to create a new quiz. Returns the ObjectID of the document.
pub async fn try_submit_quiz(col: &Collection<Quiz>, quiz: &Quiz) -> Result<[u8; 12], StatusCode> {
    // Validate the quiz
    let choice_count = quiz.choices.len();
    if usize::from(quiz.answer) >= choice_count || !(1..=25).contains(&choice_count) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Attempt to submit to MongoDB
    let InsertOneResult { inserted_id, .. } = col.insert_one(quiz, None).await.map_err(|err| {
        use mongodb::error::{ErrorKind, WriteError, WriteFailure};
        match *err.kind {
            ErrorKind::Write(WriteFailure::WriteError(WriteError { code: 11000, .. })) => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;

    // Attempt to parse as ObjectID
    let oid = inserted_id.as_object_id().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(oid.bytes())
}
