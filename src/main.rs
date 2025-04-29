mod models;
mod handlers {
    pub mod files_handler;
    pub mod tags_handler;
}
mod routes {
    pub mod folder;
}

use std::env;
use std::time::SystemTime;
use actix_web::{web, App, HttpServer, HttpResponse, Responder, get, post};
use actix_cors::Cors;
use sqlx::PgPool;
use dotenvy::dotenv;
use aws_sdk_s3::{Client, Config};
use aws_sdk_s3::config::{Credentials, Region};
use futures_util::stream::StreamExt as _;
use uuid::Uuid;
use actix_multipart::Multipart;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::presigning::{PresigningConfig};
use std::time::Duration;

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

#[post("/upload-file")]
async fn generate_presigned_url() -> impl Responder {
    let access_key = env::var("AWS_ACCESS_KEY_ID").unwrap();
    let secret_key = env::var("AWS_SECRET_ACCESS_KEY").unwrap();
    let region = env::var("AWS_REGION").unwrap_or_else(|_| "us-west-2".to_string());
    let bucket_name = env::var("MY_BUCKET_NAME").expect("MY_BUCKET_NAME must be set");

    let credentials = aws_sdk_s3::config::Credentials::new(
        access_key,
        secret_key,
        None,
        None,
        "static",
    );

    let config = aws_sdk_s3::Config::builder()
        .region(Region::new(region))
        .credentials_provider(credentials)
        .build();

    let client = Client::from_conf(config);

    let filename = "example.jpg";

    let presigning_config = match PresigningConfig::expires_in(Duration::from_secs(3600)) {
        Ok(cfg) => cfg,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Invalid expiration config: {}", e)),
    };

    let presigned_url = client
        .put_object()
        .bucket(&bucket_name)
        .key(filename)
        .presigned(presigning_config)
        .await;

    match presigned_url {
        Ok(presigned_request) => {
            let url = presigned_request.uri().to_string();
            HttpResponse::Ok().json(url)
        },
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to generate presigned URL: {}", e)),
    }
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
            .service(generate_presigned_url)
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
