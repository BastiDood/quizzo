use db::Database;
use hyper::{body, Body, HeaderMap, Response, StatusCode};
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

pub async fn try_respond(body: Body, headers: &mut HeaderMap, db: &Database) -> Result<Response<Body>, StatusCode> {
    // Ensure that the request is CORS-enabled
    use hyper::header::{HeaderValue, ACCESS_CONTROL_ALLOW_CREDENTIALS, ORIGIN};
    let origin = headers.remove(ORIGIN).ok_or(StatusCode::BAD_REQUEST)?;

    // Retrieve the session from the cookie
    let session = super::util::session::extract_session(headers)?;
    let oid = db::ObjectId::parse_str(session).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Check database if user ID is present
    let user = db
        .get_session(oid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?
        .as_user()
        .ok_or(StatusCode::FORBIDDEN)?;

    // Finally parse the JSON form submission
    use body::Buf;
    let reader = body::aggregate(body).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.reader();
    let quiz = serde_json::from_reader(reader).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Submit the quiz to the database
    use alloc::vec::Vec;
    let submission = Submission { id: user, quiz };
    let oid: Vec<_> = try_submit_quiz(db, &submission).await?.into();

    let mut res = Response::new(oid.into());
    *res.status_mut() = StatusCode::CREATED;

    let head = res.headers_mut();
    assert!(!head.append(ORIGIN, origin));
    assert!(!head.append(ACCESS_CONTROL_ALLOW_CREDENTIALS, HeaderValue::from_static("true")));
    Ok(res)
}
