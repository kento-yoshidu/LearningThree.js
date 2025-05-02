use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct UserCreateRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}
