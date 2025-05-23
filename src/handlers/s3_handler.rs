use std::time::Duration;
use actix_web::{post, web, HttpResponse, Responder};
use aws_sdk_s3::Client;
use aws_sdk_s3::error::{SdkError, ProvideErrorMetadata};
use aws_sdk_s3::presigning::PresigningConfig;
use serde::Deserialize;
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

pub async fn delete_image_from_s3(
    client: &Client,
    bucket: &str,
    image_url: &str
) -> Result<(), String> {
    let key = image_url
        .rsplit('/')
        .next()
        .ok_or_else(|| format!("無効なURL形式: {}", image_url))?;

    match client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
    {
        Ok(_) => Ok(()),
        Err(SdkError::ServiceError(service_err)) => {
            let err = service_err.err();
            if err.code() == Some("NoSuchKey") {
                // Caution: 画像が存在しない時はとりあえず無視
                println!("存在しないキー: {}", key);
                Ok(())
            } else {
                Err(format!("S3削除失敗: {} ({:?})", key, err))
            }
        }
        Err(e) => Err(format!("S3削除失敗: {} ({:?})", key, e)),
    }
}
