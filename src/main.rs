mod models;
mod handlers {
    pub mod files_handler;
    pub mod tags_handler;
}
mod routes {
    pub mod folder;
}
use actix_web::{web, App, HttpResponse, HttpServer, Responder, get};
use actix_cors::Cors;
use sqlx::PgPool;
use std::env;
use dotenvy::dotenv;

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
            .configure(routes::folder::config)
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}

#[tokio::test]
async fn test() {
    use sqlx::{PgPool, query};

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
