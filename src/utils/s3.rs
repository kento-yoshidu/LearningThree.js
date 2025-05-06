use std::env;
use aws_sdk_s3::Client;
use aws_sdk_s3::config::{Credentials, Region};

pub fn create_s3_client() -> (Client, String, String) {
    let access_key = env::var("AWS_ACCESS_KEY_ID").unwrap();
    let secret_key = env::var("AWS_SECRET_ACCESS_KEY").unwrap();
    let region = env::var("AWS_REGION").unwrap_or_else(|_| "us-west-2".to_string());
    let bucket_name = env::var("MY_BUCKET_NAME").expect("MY_BUCKET_NAME must be set");

    let credentials = Credentials::new(access_key, secret_key, None, None, "static");

    let config = aws_sdk_s3::Config::builder()
        .region(Region::new(region.clone()))
        .credentials_provider(credentials)
        .build();

    let client = Client::from_conf(config);

    (client, bucket_name, region)
}

pub async fn verify_s3_credentials() -> String {
    let (client, _, _) = create_s3_client();

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
