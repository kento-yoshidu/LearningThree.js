use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Folder {
    pub id: i32,
    pub user_id: Option<i32>,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}
