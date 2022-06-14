#![no_std]
extern crate alloc;

pub mod error;

use alloc::boxed::Box;
use core::{num::NonZeroU64, time::Duration};
use model::{
    quiz::{Quiz, Submission},
    session::Session,
};
use mongodb::{bson::doc, Collection};

pub use mongodb::{bson::oid::ObjectId, Client as MongoClient, Database as MongoDb};

pub struct Database {
    sessions: Collection<Session>,
    quizzes: Collection<Submission>,
}

impl Database {
    pub fn new(db: &MongoDb) -> Self {
        Self { sessions: db.collection("sessions"), quizzes: db.collection("quizzes") }
    }

    pub async fn create_session(&self, nonce: u64) -> error::Result<ObjectId> {
        use mongodb::results::InsertOneResult;
        let session = Session::Pending { nonce };
        let InsertOneResult { inserted_id, .. } = self.sessions.insert_one(session, None).await?;
        inserted_id.as_object_id().ok_or(error::Error::Fatal)
    }

    pub async fn upgrade_session(
        &self,
        id: ObjectId,
        user: NonZeroU64,
        access: Box<str>,
        refresh: Box<str>,
        expires: Duration,
    ) -> error::Result<bool> {
        use mongodb::bson::DateTime;
        let date = DateTime::now().to_system_time().checked_add(expires).ok_or(error::Error::TimeOverflow)?;
        let session = Session::Valid { user, access, refresh, expires: date.into() };

        use mongodb::results::UpdateResult;
        let UpdateResult { matched_count, modified_count, upserted_id, .. } =
            self.sessions.replace_one(doc! { "_id": id }, session, None).await?;
        Ok(matched_count == modified_count && upserted_id.is_none())
    }

    pub async fn get_session(&self, id: ObjectId) -> error::Result<Option<Session>> {
        let maybe_session = self.sessions.find_one(doc! { "_id": id }, None).await?;
        Ok(maybe_session)
    }

    pub async fn create_quiz(&self, quiz: &Submission) -> error::Result<ObjectId> {
        use mongodb::results::InsertOneResult;
        let InsertOneResult { inserted_id, .. } = self.quizzes.insert_one(quiz, None).await?;
        inserted_id.as_object_id().ok_or(error::Error::Fatal)
    }

    pub async fn get_quiz(&self, user: impl Into<NonZeroU64>) -> error::Result<Option<Quiz>> {
        let id: NonZeroU64 = user.into();
        let maybe_doc = self.quizzes.find_one(doc! { "_id": id.get() as i64 }, None).await?;
        Ok(maybe_doc.map(|doc| doc.quiz))
    }
}
