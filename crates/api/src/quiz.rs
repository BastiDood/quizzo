use alloc::vec::Vec;
use db::QuizzoDatabase;
use hyper::{
    body::{self, Buf},
    Body, Request, StatusCode,
};
use model::quiz::Quiz;

/// Attempts to create a new quiz. Returns the ObjectID of the document.
async fn try_submit_quiz(db: &QuizzoDatabase, quiz: &Quiz) -> Result<[u8; 12], StatusCode> {
    let choice_count = quiz.choices.len();
    if usize::from(quiz.answer) >= choice_count || !(1..=25).contains(&choice_count) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let oid = db.create_quiz(quiz).await.unwrap();
    Ok(oid.bytes())
}

pub async fn try_respond(db: &QuizzoDatabase, req: Request<Body>) -> Result<Vec<u8>, StatusCode> {
    let reader = body::aggregate(req.into_body())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .reader();
    let quiz = serde_json::from_reader(reader).map_err(|_| StatusCode::BAD_REQUEST)?;
    let oid = try_submit_quiz(db, &quiz).await?;
    Ok(oid.into())
}
