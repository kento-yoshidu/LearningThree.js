use serde::Serialize;
use actix_web::{web, get, HttpResponse, Responder};
use sqlx::PgPool;
use crate::models::{Photo, Folder, Breadcrumb};

#[derive(Serialize, Debug)]
struct FolderContents {
    folder: Folder,
    photos: Vec<Photo>,
    child_folders: Vec<Folder>,
    breadcrumbs: Vec<Breadcrumb>,
}

#[get("/files/{folder_id}")]
pub async fn get_folder_contents(folder_id: web::Path<i32>, db: web::Data<PgPool>) -> impl Responder {
    let folder_id = folder_id.into_inner();

    let folder_rows = sqlx::query!(
        "SELECT id, user_id, name, description, parent_id FROM folders WHERE id = $1",
        folder_id
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

    let photo_rows = sqlx::query!(
        "SELECT id, user_id, title, description, image_path FROM photos WHERE folder_id = $1",
        folder_id
    )
    .fetch_all(db.get_ref())
    .await;

    let photos: Vec<Photo> = match photo_rows {
        Ok(rows) => rows.into_iter().map(|row| Photo {
            id: row.id,
            user_id: row.user_id,
            title: Some(row.title),
            description: row.description,
            image_path: row.image_path,
            folder_id: Some(folder_id.to_string()),
        }).collect(),
        Err(_) => return HttpResponse::InternalServerError().body("Error fetching photos"),
    };

    let child_folder_rows = sqlx::query!(
        "SELECT id, user_id, name, description, parent_id FROM folders WHERE parent_id = $1",
        folder_id
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
