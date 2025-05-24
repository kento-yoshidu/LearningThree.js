use serde::{Serialize, Deserialize};

#[derive(Serialize, Debug)]
pub struct Folder {
    pub id: i32,
    pub user_id: Option<i32>,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub total_photo_count: Option<usize>,
}

#[derive(Deserialize)]
pub struct FolderCreateRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct FolderUpdateRequest {
    pub name: String,
    pub description: Option<String>,
    pub folder_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct FolderDeleteRequest {
    pub ids: Vec<i32>,
}
