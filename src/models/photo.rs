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
    pub user_id: Option<i32>,
    pub title: Option<String>,
    pub folder_id: Option<String>,
    pub folder_name: Option<String>,
    pub description: Option<String>,
    pub image_path: String,
    #[serde(serialize_with = "serialize_datetime")]
    pub uploaded_at: OffsetDateTime,
    pub tags: Vec<TagResponse>,
}

#[derive(Debug, Deserialize)]
pub struct PhotoUpdateRequest {
    pub id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
}
