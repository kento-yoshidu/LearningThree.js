use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Tag {
    pub id: i32,
    pub photo_id: Option<i32>,
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
