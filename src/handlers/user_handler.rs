use actix_web::{post, web, HttpResponse, Responder};
use bcrypt::verify;
use chrono::{Utc, Duration};
use jsonwebtoken::{encode, Header, EncodingKey};

use crate::models::{user::{Claims, LoginRequest, UserCreateRequest}, User};

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

#[post("signin")]
pub async fn signin(db_pool: web::Data<sqlx::PgPool>, form: web::Json<LoginRequest>) -> impl Responder {
    let user = sqlx::query_as::<_, User>(
        "SELECT
            id,
            name,
            email,
            password_hash,
            root_folder
        FROM
            users
        WHERE
            email = $1"
    )
    .bind(&form.email)
    .fetch_optional(db_pool.get_ref())
    .await;

    let user = match user {
        Ok(Some(u)) => u,
        _ => return HttpResponse::Unauthorized().body("ユーザーが見つかりません"),
    };

    // パスワード照合
    let is_valid = verify(&form.password, &user.password_hash).unwrap_or(false);
    if !is_valid {
        return HttpResponse::Unauthorized().body("パスワードが間違っています");
    }

    // JWT生成
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .unwrap()
        .timestamp();

    let claims = Claims {
        sub: user.id.to_string(),
        root_folder: user.root_folder.unwrap(),
        exp: expiration as usize,
    };

    const SECRET: &[u8] = b"secret";

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(SECRET))
        .unwrap();

    HttpResponse::Ok().json(serde_json::json!({ "token": token }))
}
