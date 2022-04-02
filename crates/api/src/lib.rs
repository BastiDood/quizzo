use http::StatusCode;
use model::Quiz;
use mongodb::{results::InsertOneResult, Collection};

/// Attempts to create a new quiz. Returns the ObjectID of the document.
pub async fn try_submit_quiz(col: &Collection<Quiz>, quiz: &Quiz) -> Result<[u8; 12], StatusCode> {
    let InsertOneResult { inserted_id, .. } = col.insert_one(quiz, None).await.map_err(|err| {
        use mongodb::error::{ErrorKind, WriteError, WriteFailure};
        match *err.kind {
            ErrorKind::Write(WriteFailure::WriteError(WriteError { code: 11000, .. })) => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    })?;
    let oid = inserted_id.as_object_id().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(oid.bytes())
}
