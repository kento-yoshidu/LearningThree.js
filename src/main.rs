use actix_web::{web, App, HttpResponse, HttpServer, Responder, get};
use actix_cors::Cors;
use sqlx::{PgPool, query};
use serde::Serialize;
use std::env;
use dotenvy::dotenv;

#[derive(Serialize, Debug)]
struct Photo {
    id: i32,
    user_id: Option<i32>,
    title: Option<String>,
    folder_id: Option<String>,
    description: Option<String>,
    image_path: String,
}

#[derive(Serialize, Debug)]
struct Folder {
    id: i32,
    user_id: Option<i32>,
    name: String,
    description: Option<String>,
    parent_id: Option<i32>,
}

#[derive(serde::Serialize, Debug)]
struct Breadcrumb {
    id: Option<i32>,
    name: Option<String>,
}

#[derive(Serialize, Debug)]
struct FolderContents {
    folder: Folder,
    photos: Vec<Photo>,
    child_folders: Vec<Folder>,
    breadcrumbs: Vec<Breadcrumb>,
}

#[get("/files/{folder_id}")]
async fn get_folder_contents(folder_id: web::Path<i32>, db: web::Data<PgPool>) -> impl Responder {
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

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello World!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    let pool_data = web::Data::new(pool);

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600),
            )
            .app_data(pool_data.clone())
            .service(hello)
            .service(get_folder_contents)
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}

#[tokio::test]
async fn test() {
    dotenvy::from_filename(".env.test").ok();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL_TEST must be set");
    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to DB");

    let photo = query!(
        "SELECT id, title, description, image_path, folder_id FROM photos WHERE id = $1",
        1
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch test photo");

    assert_eq!(photo.title, "admin_photo_1");
    assert_eq!(photo.description, Some("admin photo 1".to_string()));
    assert_eq!(photo.image_path, "/images/1.jpg");
    assert_eq!(photo.folder_id, Some(1));
}

#[tokio::test]
async fn test_existing_breadcrumbs() {
    dotenvy::from_filename(".env.test").ok();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&db_url).await.expect("Failed to connect to DB");

    let app = actix_web::test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(get_folder_contents),
    )
    .await;

    let req = actix_web::test::TestRequest::get()
        .uri("/files/3")
        .to_request();

    let resp = actix_web::test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body = actix_web::test::read_body(resp).await;
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let breadcrumbs = result["breadcrumbs"].as_array().unwrap();
    let names: Vec<&str> = breadcrumbs
        .iter()
        .map(|b| b["name"].as_str().unwrap())
        .collect();

    assert_eq!(names, vec!["admin", "admin_1", "admin_1_1"]);
}
