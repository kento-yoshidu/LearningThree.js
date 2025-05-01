use serde::{Serialize, Deserialize};

#[derive(Serialize, Debug)]
pub struct Folder {
    pub id: i32,
    pub user_id: Option<i32>,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

#[derive(Deserialize)]
pub struct FolderCreateRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}
