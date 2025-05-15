use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use sqlx::PgPool;
use crate::models::{tag::TagResponse, Breadcrumb, Folder, Photo, Tag};
use crate::handlers::auth_handler::extract_user_from_jwt;

#[derive(Serialize, Debug)]
struct FolderContents {
    folder: Folder,
    photos: Vec<Photo>,
    child_folders: Vec<Folder>,
    breadcrumbs: Vec<Breadcrumb>,
}

#[derive(Deserialize)]
pub struct PhotoCreateRequest {
    pub image_path: String,
    pub title: Option<String>,
    pub folder_id: Option<i32>,
    pub description: Option<String>,
}

#[get("/files/{folder_id}")]
pub async fn get_folder_contents(
    req: HttpRequest,
    path: web::Path<i32>,
    db: web::Data<PgPool>
) -> impl Responder {
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let folder_id = path.into_inner();

    let folder_rows = sqlx::query!(
        "SELECT
            id,
            user_id,
            name,
            description,
            parent_id
        FROM
            folders
        WHERE
            id = $1 AND
            user_id = $2",
        folder_id,
        claims.user_id,
    )
    .fetch_all(db.get_ref())
    .await;

    let folder = match folder_rows {
        Ok(rows) if !rows.is_empty() => Folder {
            id: rows[0].id,
            user_id: rows[0].user_id,
            name: rows[0].name.clone(),
            description: rows[0].description.clone(),
            parent_id: rows[0].parent_id,
        },
        Ok(_) => return HttpResponse::NotFound().body("Folder not found"),
        Err(_) => return HttpResponse::InternalServerError().body("Error fetching folder"),
    };

    // 子フォルダー
    let child_folder_rows = sqlx::query!(
        "SELECT
            id,
            user_id,
            name,
            description,
            parent_id
        FROM
            folders
        WHERE
            parent_id = $1 AND
            user_id = $2",
        folder_id,
        claims.user_id,
    )
    .fetch_all(db.get_ref())
    .await;

    let child_folders: Vec<Folder> = match child_folder_rows {
        Ok(rows) => rows.into_iter().map(|row| Folder {
            id: row.id,
            user_id: row.user_id,
            name: row.name.clone(),
            description: row.description.clone(),
            parent_id: row.parent_id,
        }).collect(),
        Err(_) => return HttpResponse::InternalServerError().body("Error fetching child folders"),
    };

    // 写真データ
    let photo_rows = sqlx::query!(
        "SELECT
            photos.id,
            photos.user_id,
            photos.title,
            photos.description,
            photos.image_path,
            photos.uploaded_at,
            folders.name AS folder_name
        FROM
            photos
        LEFT JOIN
            folders ON photos.folder_id = folders.id
        WHERE
            photos.folder_id = $1 AND
            photos.user_id = $2",
        folder_id,
        claims.user_id,
    )
    .fetch_all(db.get_ref())
    .await;

    let rows = match photo_rows {
        Ok(rows) => rows,
        Err(_) => return HttpResponse::InternalServerError().body("Error fetching photos"),
    };

    let photo_ids: Vec<i32> = rows.iter().map(|row| row.id).collect();

    // photo_tags テーブルと tags テーブルを結合して取得
    let photo_tag_rows = sqlx::query!(
        "
        SELECT
            photo_tag_relations.photo_id,
            tags.id AS tag_id,
            tags.tag
        FROM
            photo_tag_relations
        INNER JOIN
            tags ON photo_tag_relations.tag_id = tags.id
        WHERE
            photo_tag_relations.photo_id = ANY($1)
        ",
        &photo_ids
    )
    .fetch_all(db.get_ref())
    .await;

    let mut tag_map: HashMap<i32, Vec<TagResponse>> = HashMap::new();

    if let Ok(tag_rows) = photo_tag_rows {
        for tag_row in tag_rows {
            tag_map
                .entry(tag_row.photo_id)
                .or_default()
                .push(TagResponse {
                    id: tag_row.tag_id,
                    tag: tag_row.tag,
                });
        }
    }


    let photos: Vec<Photo> = rows.into_iter().map(|row| Photo {
        id: row.id,
        user_id: row.user_id,
        title: Some(row.title),
        description: row.description,
        image_path: row.image_path,
        uploaded_at: row.uploaded_at,
        folder_id: Some(folder_id.to_string()),
        folder_name: Some(row.folder_name),
        tags: tag_map.remove(&row.id).unwrap_or_default(),
    }).collect();

    // パンくずリスト
    let breadcrumb_rows = sqlx::query!(
        "
        WITH RECURSIVE breadcrumb AS (
            SELECT id, name, parent_id
            FROM folders
            WHERE id = $1

            UNION ALL

            SELECT f.id, f.name, f.parent_id
            FROM folders f
            JOIN breadcrumb b ON f.id = b.parent_id
        )
        SELECT
            id,
            name
        FROM
            breadcrumb
        ORDER BY parent_id NULLS FIRST;
        ",
        folder_id
    )
    .fetch_all(db.get_ref())
    .await;

    let breadcrumbs: Vec<Breadcrumb> = match breadcrumb_rows {
        Ok(rows) => rows.into_iter().map(|row| Breadcrumb {
            id: row.id,
            name: row.name,
        }).collect(),
        Err(_) => return HttpResponse::InternalServerError().body("Error fetching breadcrumbs"),
    };

    HttpResponse::Ok().json(FolderContents {
        folder,
        photos,
        child_folders,
        breadcrumbs,
    })
}

#[get("/search")]
pub async fn get_all_photos(
    req: HttpRequest,
    db: web::Data<PgPool>
) -> impl Responder {
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let photo_rows = sqlx::query!(
        "SELECT
            photos.id,
            photos.user_id,
            photos.title,
            photos.description,
            photos.image_path,
            photos.uploaded_at,
            photos.folder_id,
            folders.name AS folder_name
        FROM
            photos
        LEFT JOIN
            folders ON photos.folder_id = folders.id
        WHERE
            photos.user_id = $1",
        claims.user_id,
    )
    .fetch_all(db.get_ref())
    .await;

    let rows = match photo_rows {
        Ok(rows) => rows,
        Err(_) => return HttpResponse::InternalServerError().body("Error fetching photos"),
    };

    let photo_ids: Vec<i32> = rows.iter().map(|row| row.id).collect();

    let photo_tag_rows = sqlx::query!(
        "SELECT
            ptr.photo_id,
            t.id AS tag_id,
            t.tag
        FROM
            photo_tag_relations ptr
        INNER JOIN
            tags t ON ptr.tag_id = t.id
        WHERE
            ptr.photo_id = ANY($1)",
        &photo_ids
    )
    .fetch_all(db.get_ref())
    .await;

    let mut tag_map: HashMap<i32, Vec<TagResponse>> = HashMap::new();

    if let Ok(tag_rows) = photo_tag_rows {
        for row in tag_rows {
            tag_map
                .entry(row.photo_id)
                .or_default()
                .push(TagResponse {
                    id: row.tag_id,
                    tag: row.tag,
                });
        }
    }

    let photos: Vec<Photo> = rows.into_iter().map(|row| Photo {
        id: row.id,
        user_id: row.user_id,
        title: Some(row.title),
        description: row.description,
        image_path: row.image_path,
        uploaded_at: row.uploaded_at,
        folder_id: row.folder_id.map(|id| id.to_string()),
        folder_name: Some(row.folder_name),
        tags: tag_map.remove(&row.id).unwrap_or_default(),
    }).collect();

    HttpResponse::Ok().json(photos)
}
