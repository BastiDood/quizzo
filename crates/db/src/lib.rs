#![no_std]

pub mod error;

use core::num::NonZeroU64;
use model::quiz::Submission;
use mongodb::{bson::doc, results::InsertOneResult, Collection};

pub use mongodb::bson::oid::ObjectId;
pub use mongodb::Client as MongoClient;
pub use mongodb::Database as MongoDb;

pub type Session = Option<NonZeroU64>;

pub struct Database {
    sessions: Collection<Session>,
    quizzes: Collection<Submission>,
}

impl Database {
    pub fn new(db: &MongoDb) -> Self {
        Self { sessions: db.collection("sessions"), quizzes: db.collection("quizzes") }
    }

    pub async fn create_session(&self) -> error::Result<ObjectId> {
        let InsertOneResult { inserted_id, .. } = self.sessions.insert_one(None, None).await?;
        inserted_id.as_object_id().ok_or(error::Error::Fatal)
    }

    pub async fn upgrade_session(&self, session: ObjectId, user: impl Into<NonZeroU64>) -> error::Result<()> {
        let old = self.sessions.find_one_and_replace(doc! { "_id": session }, Some(user.into()), None).await?;
        assert!(old.is_none());
        Ok(())
    }

    pub async fn get_session(&self, session: ObjectId) -> error::Result<Option<Session>> {
        let maybe_session = self.sessions.find_one(doc! { "_id": session }, None).await?;
        Ok(maybe_session)
    }

    pub async fn create_quiz(&self, quiz: &Submission) -> error::Result<ObjectId> {
        let InsertOneResult { inserted_id, .. } = self.quizzes.insert_one(quiz, None).await?;
        inserted_id.as_object_id().ok_or(error::Error::Fatal)
    }
}
