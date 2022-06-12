use db::Database;
use hyper::StatusCode;
use model::quiz::Submission;

/// Attempts to create a new quiz. Returns the ObjectID of the document.
pub async fn try_submit_quiz(db: &Database, sub: &Submission) -> Result<[u8; 12], StatusCode> {
    let choice_count = sub.quiz.choices.len();
    if usize::from(sub.quiz.answer) >= choice_count || !(1..=25).contains(&choice_count) {
        return Err(StatusCode::BAD_REQUEST);
    }

    match db.create_quiz(sub).await {
        Ok(oid) => Ok(oid.bytes()),
        Err(db::error::Error::AlreadyExists) => Err(StatusCode::FORBIDDEN),
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
