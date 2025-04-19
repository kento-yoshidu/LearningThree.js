use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use serde::Serialize;
use sqlx::postgres::PgPool;
use std::env;
use dotenvy::dotenv;

#[derive(Serialize, Debug)]
struct Photo {
    id: i32,
    user_id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    image_path: String,
}

#[get("/photos")]
async fn get_photos(db: web::Data<PgPool>) -> impl Responder {
    let rows = sqlx::query!(
        "SELECT
            id,
            user_id::TEXT as user_id,
            title,
            description,
            image_path
        FROM
            photos"
    )
    .fetch_all(db.get_ref())
    .await
    .unwrap();

    let photos: Vec<Photo> = rows
        .into_iter()
        .map(|row| Photo {
            id: row.id,
            user_id: row.user_id,
            title: row.title,
            description: row.description,
            image_path: row.image_path,
        })
        .collect();

    web::Json(photos)
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
            .app_data(pool_data.clone())
            .service(hello)
            .service(get_photos)
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}
