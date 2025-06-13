use serde::{Serialize, Deserialize};
use serde_with::serde_as;
use super::tag::TagResponse;
use time::OffsetDateTime;

fn serialize_datetime<S>(
    datetime: &OffsetDateTime,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let format =
        time::format_description::parse("[year]-[month]-[day]T[hour]:[minute]")
            .map_err(serde::ser::Error::custom)?;

    let s = datetime.format(&format).map_err(serde::ser::Error::custom)?;

    serializer.serialize_str(&s)
}

#[serde_as]
#[derive(Serialize, Debug)]
pub struct Photo {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub folder_id: i32,
    pub folder_name: String,
    pub description: Option<String>,
    pub image_path: String,
    #[serde(serialize_with = "serialize_datetime")]
    pub uploaded_at: OffsetDateTime,
    pub size_in_bytes: i64,
    pub width: i32,
    pub height: i32,
    pub tags: Vec<TagResponse>,
}

#[derive(Deserialize)]
pub struct PhotoUploadRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub folder_id: Option<i32>,
    pub image_path: String,
    pub size_in_bytes: i64,
}

#[derive(Debug, Deserialize)]
pub struct PhotoUpdateRequest {
    pub id: i32,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PhotoMoveRequest {
    pub ids: Vec<i32>,
    pub folder_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct PhotoDeleteRequest {
    pub ids: Vec<i32>,
}

#[derive(Debug, Deserialize)]
pub struct PhotoSearchRequest {
    pub tags: String,
}

#[derive(Serialize)]
pub struct PhotoWrapper {
    pub data: Vec<PhotoResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PhotoResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub image_path: String,
    pub folder_id: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Deserialize)]
pub struct TagAddRequest {
    pub photo_ids: Vec<i32>,
    pub tag_ids: Vec<i32>,
}
