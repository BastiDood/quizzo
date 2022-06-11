use alloc::vec::Vec;
use db::Database;
use hyper::{
    body::{self, Buf},
    Body, Request, StatusCode,
};
use model::quiz::Submission;

/// Attempts to create a new quiz. Returns the ObjectID of the document.
async fn try_submit_quiz(db: &Database, sub: &Submission) -> Result<[u8; 12], StatusCode> {
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

pub async fn try_respond(db: &Database, req: Request<Body>) -> Result<Vec<u8>, StatusCode> {
    let reader = body::aggregate(req.into_body()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.reader();
    let quiz = serde_json::from_reader(reader).map_err(|_| StatusCode::BAD_REQUEST)?;
    let oid = try_submit_quiz(db, &quiz).await?;
    Ok(oid.into())
}
