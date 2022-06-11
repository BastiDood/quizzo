#![no_std]

pub mod error;

use core::num::NonZeroU64;
use model::quiz::Submission;
use mongodb::{
    bson::{doc, oid::ObjectId},
    results::InsertOneResult,
    Collection,
};

pub use mongodb::Client as MongoClient;
pub use mongodb::Database as MongoDb;

pub type Session = Option<NonZeroU64>;

pub struct Database {
    sessions: Collection<Session>,
    quizzes: Collection<Submission>,
}

impl Database {
    pub fn new(db: &MongoDb) -> Self {
        Self {
            sessions: db.collection("sessions"),
            quizzes: db.collection("quizzes"),
        }
    }

    pub async fn create_session(&self) -> error::Result<ObjectId> {
        let InsertOneResult { inserted_id, .. } = self.sessions.insert_one(None, None).await?;
        inserted_id.as_object_id().ok_or(error::Error::Fatal)
    }

    pub async fn upgrade_session(&self, session: ObjectId, user: impl Into<NonZeroU64>) -> error::Result<Session> {
        self.sessions
            .find_one_and_replace(doc! { "_id": session }, Some(user.into()), None)
            .await?
            .ok_or(error::Error::NoDocument)
    }

    pub async fn create_quiz(&self, quiz: &Submission) -> error::Result<ObjectId> {
        let InsertOneResult { inserted_id, .. } = self.quizzes.insert_one(quiz, None).await?;
        inserted_id.as_object_id().ok_or(error::Error::Fatal)
    }
}
