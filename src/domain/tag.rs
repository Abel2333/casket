use uuid::Uuid;

pub type TagId = Uuid;

pub struct Tag {
    pub id: TagId,
    pub name: String,
}
