use std::collections::HashMap;

use serde::Serialize;
use actix_web::{web, get, HttpResponse, Responder};
use sqlx::PgPool;
use crate::models::{tag::TagResponse, Breadcrumb, Folder, Photo, Tag};

#[derive(Serialize, Debug)]
struct FolderContents {
    folder: Folder,
    photos: Vec<Photo>,
    child_folders: Vec<Folder>,
    breadcrumbs: Vec<Breadcrumb>,
}

#[get("/files/{folder_id}/{user_id}")]
pub async fn get_folder_contents(path: web::Path<(i32, i32)>, db: web::Data<PgPool>) -> impl Responder {
    let (folder_id, user_id) = path.into_inner();

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
        user_id,
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
        user_id,
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
            id,
            user_id,
            title,
            description,
            image_path
        FROM
            photos
        WHERE
            folder_id = $1 AND
            user_id = $2",
        folder_id,
        user_id,
    )
    .fetch_all(db.get_ref())
    .await;

    let rows = match photo_rows {
        Ok(rows) => rows,
        Err(_) => return HttpResponse::InternalServerError().body("Error fetching photos"),
    };

    let photo_ids: Vec<i32> = rows.iter().map(|row| row.id).collect();

    let photo_tag_rows = sqlx::query!(
        "SELECT photo_id, tag, id FROM photo_tags WHERE photo_id = ANY($1)",
        &photo_ids
    )
    .fetch_all(db.get_ref())
    .await;

    let mut tag_map: HashMap<i32, Vec<TagResponse>> = HashMap::new();

    if let Ok(tag_rows) = photo_tag_rows {
        for tag_row in tag_rows {
            if let Some(photo_id) = tag_row.photo_id {
                let tag = Tag {
                    id: tag_row.id,
                    photo_id: tag_row.photo_id,
                    user_id: None,
                    tag: tag_row.tag,
                };
                tag_map
                    .entry(photo_id)
                    .or_default()
                    .push(tag.into());
            }
        }
    }


    let photos: Vec<Photo> = rows.into_iter().map(|row| Photo {
        id: row.id,
        user_id: row.user_id,
        title: Some(row.title),
        description: row.description,
        image_path: row.image_path,
        folder_id: Some(folder_id.to_string()),
        tags: tag_map.remove(&row.id).unwrap_or_default(),
    }).collect();

    // パンくずリスト
    let breadcrumb_rows = sqlx::query!(
        r#"
        WITH RECURSIVE breadcrumb AS (
            SELECT id, name, parent_id
            FROM folders
            WHERE id = $1

            UNION ALL

            SELECT f.id, f.name, f.parent_id
            FROM folders f
            JOIN breadcrumb b ON f.id = b.parent_id
        )
        SELECT id, name
        FROM breadcrumb
        ORDER BY parent_id NULLS FIRST;
        "#,
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
