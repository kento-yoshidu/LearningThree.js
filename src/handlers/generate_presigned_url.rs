use std::env;
use std::time::Duration;
use serde::Deserialize;
use actix_web::{post, web, HttpResponse, Responder};
use aws_sdk_s3::config::Region;
use aws_sdk_s3::Client;
use aws_sdk_s3::presigning::PresigningConfig;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct PresignRequest {
    filename: String,
}

#[post("/generate-presigned-url")]
pub async fn generate_presigned_url(
    req: web::Json<PresignRequest>
) -> impl Responder {
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
        .region(Region::new(region.clone()))
        .credentials_provider(credentials)
        .build();

    let client = Client::from_conf(config);

    let filename = format!("{}-{}", Uuid::new_v4(), req.filename);

    let presigning_config = match PresigningConfig::expires_in(Duration::from_secs(300)) {
        Ok(cfg) => cfg,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Invalid expiration config: {}", e)),
    };

    let presigned_url = client
        .put_object()
        .bucket(&bucket_name)
        .key(filename.clone())
        .presigned(presigning_config)
        .await;

    match presigned_url {
        Ok(presigned_request) => {
            HttpResponse::Ok().json(serde_json::json!({
                "presigned_url": presigned_request.uri().to_string(),
                "public_url": format!("https://{}.s3.{}.amazonaws.com/{}", bucket_name, region, filename)
            }))
        },
        Err(e) => HttpResponse::InternalServerError().body(format!("Failed to generate presigned URL: {}", e)),
    }
}
