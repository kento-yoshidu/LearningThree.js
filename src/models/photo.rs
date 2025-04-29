use serde::Serialize;

use super::tag::TagResponse;

#[derive(Serialize, Debug)]
pub struct Photo {
    pub id: i32,
    pub user_id: Option<i32>,
    pub title: Option<String>,
    pub folder_id: Option<String>,
    pub description: Option<String>,
    pub image_path: String,
    pub tags: Vec<TagResponse>,
}
