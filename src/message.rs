use std::fmt;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum FileType {
    Folder,
    Photo,
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileType::Folder => write!(f, "フォルダー"),
            FileType::Photo => write!(f, "写真"),
        }
    }
}

#[derive(Debug, Serialize)]
pub enum AppSuccess {
    CreatedFolder,
    Updated(FileType),
    Deleted(FileType),
    UploadedPhoto,
}

impl AppSuccess {
    pub fn message(&self) -> String {
        match self {
            AppSuccess::CreatedFolder => "Folder was created.".to_string(),
            AppSuccess::Updated(file_type) => format!("{file_type}の更新が成功しました。"),
            AppSuccess::Deleted(file_type) => format!("{file_type}の削除が成功しました。"),
            AppSuccess::UploadedPhoto=> "写真のアップロードが成功しました。".to_string(),
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
