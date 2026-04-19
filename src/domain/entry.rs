use crate::domain::{
    secret::{DecryptedSecretField, SecretDraftField},
    tag::Tag,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub type EntryId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Journal,
    Note,
    Bookmark,
    Credential,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: EntryId,
    pub kind: EntryKind,
    pub title: String,
    pub body: String,
    pub is_favorite: bool,
    pub is_archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct EntryDraft {
    pub id: Option<EntryId>,
    pub kind: EntryKind,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub secret_fields: Vec<SecretDraftField>,
}

#[derive(Debug, Clone, Default)]
pub struct EntryFilter {
    pub kind: Option<EntryKind>,
    pub include_archived: bool,
    pub search_term: Option<String>,
    pub tag: Option<String>,
}

pub struct EntryDetail {
    pub entry: Entry,
    pub tags: Vec<Tag>,
    pub secret_fields: Vec<DecryptedSecretField>,
}
