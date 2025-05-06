use std::time::Duration;
use serde::Deserialize;
use actix_web::{post, web, HttpResponse, Responder};
use aws_sdk_s3::presigning::PresigningConfig;
use uuid::Uuid;

use crate::utils::s3::create_s3_client;

#[derive(Deserialize)]
pub struct PresignRequest {
    filename: String,
}

#[post("/generate-presigned-url")]
pub async fn generate_presigned_url(
    req: web::Json<PresignRequest>
) -> impl Responder {
    let filename = format!("{}-{}", Uuid::new_v4(), req.filename);

    let presigning_config = match PresigningConfig::expires_in(Duration::from_secs(300)) {
        Ok(cfg) => cfg,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Invalid expiration config: {}", e)),
    };

    let (client, bucket_name, region) = create_s3_client();

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
