use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use sqlx::PgPool;
use dotenvy::dotenv;
use std::env;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello World!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&db_url).await.expect("Failed to connect to DB");

    HttpServer::new(|| {
        App::new()
            .service(hello)
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}
