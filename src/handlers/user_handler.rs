use actix_web::{post, web, HttpResponse, Responder};
use bcrypt;

use crate::models::user::UserCreateRequest;

#[post("signup")]
async fn signup(db_pool: web::Data<sqlx::PgPool>, paylod: web::Json<UserCreateRequest>) -> impl Responder {
    let hashed = bcrypt::hash(&paylod.password, bcrypt::DEFAULT_COST).unwrap();

    let result = sqlx::query!(
        "INSERT INTO users (name, email, password_hash)
        VALUES ($1, $2, $3)",
        paylod.name,
        paylod.email,
        hashed
    )
    .execute(db_pool.get_ref())
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().body("ユーザー登録完了"),
        Err(e) => {
            eprintln!("DB保存エラー: {:?}", e);
            HttpResponse::InternalServerError().body("保存失敗")
        }
    }
}
