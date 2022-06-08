#![no_std]

use core::num::NonZeroU64;
use model::quiz::Quiz;
use mongodb::{
    bson::{doc, oid::ObjectId},
    results::InsertOneResult,
    Collection, Database,
};

pub type Session = Option<NonZeroU64>;

pub struct QuizzoDatabase {
    sessions: Collection<Session>,
    quizzes: Collection<Quiz>,
}

impl QuizzoDatabase {
    pub fn new(db: &Database) -> Self {
        Self {
            sessions: db.collection("sessions"),
            quizzes: db.collection("quizzes"),
        }
    }

    pub async fn create_session(&self) -> mongodb::error::Result<ObjectId> {
        let InsertOneResult { inserted_id, .. } = self.sessions.insert_one(None, None).await?;
        let id = inserted_id.as_object_id().unwrap();
        Ok(id)
    }

    pub async fn upgrade_session(&self, session: ObjectId, user: impl Into<NonZeroU64>) -> mongodb::error::Result<Session> {
        let old = self
            .sessions
            .find_one_and_replace(doc! { "_id": session }, Some(user.into()), None)
            .await?
            .unwrap();
        Ok(old)
    }

    pub async fn create_quiz(&self, quiz: &Quiz) -> mongodb::error::Result<ObjectId> {
        let InsertOneResult { inserted_id, .. } = self.quizzes.insert_one(quiz, None).await?;
        let id = inserted_id.as_object_id().unwrap();
        Ok(id)
    }
}
