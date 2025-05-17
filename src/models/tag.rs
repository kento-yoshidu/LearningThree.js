use serde::{Serialize, Deserialize};

#[derive(Serialize, Debug)]
pub struct Tag {
    pub id: i32,
    pub user_id: Option<i32>,
    pub tag: String,
}

#[derive(Clone, Serialize, Debug)]
pub struct TagResponse {
    pub id: i32,
    pub tag: String,
}

impl From<Tag> for TagResponse {
    fn from(tag: Tag) -> Self {
        TagResponse {
            id: tag.id,
            tag: tag.tag,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddTagRequest {
    pub photo_id: i32,
    pub tag: String,
}
