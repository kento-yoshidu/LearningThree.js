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
