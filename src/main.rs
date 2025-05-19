use actix_web::{web, App, HttpServer};


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
    App::new()
        .route("/", web::get().to(|| async { "Hello from Actix!" }))
    })
    .bind(("0.0.0.0", 8080))?  // ← ここ重要！
    .run()
    .await
}
