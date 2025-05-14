use actix_web::{App, HttpResponse, ResponseError};
use chrono::format::StrftimeItems;
use derive_more::Display;
use serde::Serialize;

#[derive(Display, Debug, Serialize)]
pub enum FileType {
    Folder,
    Photo,
}

#[derive(Debug, Serialize)]
pub enum AppSuccess {
    CreatedFolder,
    UpdatedFolder(String),
    DeletedFolder(String),
    UploadedPhoto,
}

impl AppSuccess {
    pub fn message(&self) -> String {
        match self {
            AppSuccess::CreatedFolder => "Folder was created.".to_string(),
            AppSuccess::UpdatedFolder(name) => format!("{} was updated.", name),
            AppSuccess::DeletedFolder(name) => format!("{} was deleted.", name),
            AppSuccess::UploadedPhoto=> "Photo was uploaded".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub enum AppError {
    CreateFolderFailed,
    UpdateFolderFailed,
    DeleteFailed(FileType),
    UploadFailed(FileType),
    InternalServerError,
}

impl AppError {
    pub fn message(&self) -> String {
        match self {
            AppError::CreateFolderFailed => "Failed to create folder.".to_string(),
            AppError::UpdateFolderFailed => "Failed to update folder.".to_string(),
            AppError::DeleteFailed(file_type) => format!("Failed to delete {file_type}."),
            AppError::UploadFailed(file_type) => format!("Failed to upload {file_type}."),
            AppError::InternalServerError => "Internal Server Error".to_string(),
        }
    }
}
