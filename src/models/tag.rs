use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Tag {
    pub id: i32,
    pub photo_id: Option<i32>,
    pub user_id: Option<i32>,
    pub tag: String,
}
