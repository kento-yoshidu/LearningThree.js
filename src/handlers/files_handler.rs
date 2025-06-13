use std::collections::HashMap;
use serde::Serialize;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use sqlx::PgPool;
use crate::models::{tag::TagResponse, Breadcrumb, Folder, Photo};
use crate::handlers::auth_handler::extract_user_from_jwt;
use bigdecimal::ToPrimitive;

#[derive(Serialize, Debug)]
struct FolderContents {
    folder: Folder,
    photos: Vec<Photo>,
    child_folders: Vec<Folder>,
    breadcrumbs: Vec<Breadcrumb>,
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
            total_photo_count: None,
            total_photo_size: None,
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

    let mut child_folders: Vec<Folder> = Vec::new();

    for row in child_folder_rows.unwrap_or_default() {
        let folder_ids_result = sqlx::query!(
            "
            WITH RECURSIVE all_folders AS (
                SELECT id FROM folders WHERE id = $1
                UNION ALL
                SELECT f.id
                FROM folders f
                INNER JOIN all_folders af ON f.parent_id = af.id
            )
            SELECT id FROM all_folders
            ",
            row.id
        )
        .fetch_all(db.get_ref())
        .await;

        let folder_ids: Vec<i32> = match folder_ids_result {
            Ok(rows) => rows.into_iter().map(|r| r.id.unwrap()).collect(),
            Err(_) => return HttpResponse::InternalServerError().body("Error fetching folder hierarchy"),
        };

        // フォルダID群に属する写真枚数を取得
        let photo_count_row = sqlx::query!(
            "
            SELECT COUNT(*) as count
            FROM photos
            WHERE folder_id = ANY($1) AND user_id = $2
            ",
            &folder_ids,
            claims.user_id
        )
        .fetch_one(db.get_ref())
        .await;

        let total_photo_count = match photo_count_row {
            Ok(row) => row.count.unwrap_or(0) as usize,
            Err(_) => return HttpResponse::InternalServerError().body("Error counting photos"),
        };

        let photo_size_sum_row = sqlx::query!(
            "
            SELECT COALESCE(SUM(size_in_bytes), 0) as sum_size
            FROM photos
            WHERE folder_id = ANY($1) AND user_id = $2
            ",
            &folder_ids,
            claims.user_id
        )
        .fetch_one(db.get_ref())
        .await;

        // total_photo_size を i64 に変換
        let total_photo_size = match photo_size_sum_row {
            Ok(row) => row.sum_size.and_then(|bd| bd.to_i64()).unwrap_or(0),
            Err(_) => return HttpResponse::InternalServerError().body("Error summing photo sizes"),
        };

        child_folders.push(Folder {
            id: row.id,
            user_id: row.user_id,
            name: row.name.clone(),
            description: row.description.clone(),
            parent_id: row.parent_id,
            total_photo_count: Some(total_photo_count),
            total_photo_size: Some(total_photo_size),
        });
    }

    // 写真データ
    let photo_rows = sqlx::query!(
        "SELECT
            photos.id,
            photos.user_id,
            photos.name,
            photos.description,
            photos.image_path,
            photos.uploaded_at,
            photos.size_in_bytes,
            photos.width,
            photos.height,
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
        name: row.name,
        description: row.description,
        image_path: row.image_path,
        uploaded_at: row.uploaded_at,
        folder_id: folder_id,
        size_in_bytes: row.size_in_bytes,
        folder_name: row.folder_name,
        tags: tag_map.remove(&row.id).unwrap_or_default(),
        width: row.width,
        height: row.height,
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
            photos.name,
            photos.description,
            photos.image_path,
            photos.uploaded_at,
            photos.folder_id,
            photos.size_in_bytes,
            photos.width,
            photos.height,
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
        name: row.name,
        description: row.description,
        image_path: row.image_path,
        uploaded_at: row.uploaded_at,
        folder_id: row.folder_id,
        folder_name: row.folder_name,
        size_in_bytes: row.size_in_bytes,
        tags: tag_map.remove(&row.id).unwrap_or_default(),
        width: row.width,
        height: row.height,
    }).collect();

    HttpResponse::Ok().json(photos)
}
