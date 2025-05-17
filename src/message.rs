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
    Updated(FileType),
    DeletedFolder(String),
    UploadedPhoto,
}

impl AppSuccess {
    pub fn message(&self) -> String {
        match self {
            AppSuccess::CreatedFolder => "Folder was created.".to_string(),
            AppSuccess::Updated(file_type) => format!("{file_type} was updated."),
            AppSuccess::DeletedFolder(file_name) => format!("{file_name} was deleted."),
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
    TransactionStartFailed,
}

impl AppError {
    pub fn message(&self) -> String {
        match self {
            AppError::CreateFolderFailed => "Failed to create folder.".to_string(),
            AppError::UpdateFolderFailed => "Failed to update folder.".to_string(),
            AppError::DeleteFailed(file_type) => format!("Failed to delete {file_type}."),
            AppError::UploadFailed(file_type) => format!("Failed to upload {file_type}."),
            AppError::InternalServerError => "Internal Server Error".to_string(),
            AppError::TransactionStartFailed => "Failed to start transaction".to_string(),
        }
    }
}
