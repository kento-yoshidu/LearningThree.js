mod models;
mod handlers {
    pub mod files_handler;
    pub mod folder_handler;
    pub mod tags_handler;
    pub mod generate_presigned_url;
}
mod routes {
    pub mod routes;
}

use std::env;
use std::time::SystemTime;
use actix_web::{web, App, HttpServer, Responder, get};
use actix_cors::Cors;
use sqlx::PgPool;
use dotenvy::dotenv;
use aws_sdk_s3::{Client, Config};
use aws_sdk_s3::config::{Credentials, Region};

async fn verify_s3_credentials() -> String {
    dotenv().ok();

    let access_key = env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID not set");
    let secret_key = env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_ACCESS_KEY not set");
    let region = env::var("AWS_REGION").unwrap_or_else(|_| "us-west-2".to_string());

    let credentials = Credentials::new(
        access_key,
        secret_key,
        None,
        Some(SystemTime::now()),
        "static credentials",
    );

    let config = Config::builder()
        .region(Region::new(region))
        .credentials_provider(credentials)
        .build();

    let client = Client::from_conf(config);

    match client.list_buckets().send().await {
        Ok(response) => {
            let bucket_names: Vec<String> = response.buckets().unwrap_or_default()
                .iter()
                .filter_map(|bucket| bucket.name().map(|s| s.to_string()))
                .collect();

            format!("success: Buckets: {:?}", bucket_names)
        }
        Err(e) => {
            format!("failed: {}", e)
        }
    }
}

#[get("/check-s3-auth")]
async fn check_s3_authentication() -> impl Responder {
    verify_s3_credentials().await
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
            .service(check_s3_authentication)
            .configure(routes::routes::config)
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
