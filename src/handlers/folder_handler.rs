use actix_web::{post, web, HttpRequest, HttpResponse, HttpMessage, Responder};
use serde::Deserialize;

use crate::models::user::Claims;

#[derive(Deserialize)]
pub struct FolderCreateRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

#[post("/create-folder")]
async fn create_folder(
    db_pool: web::Data<sqlx::PgPool>,
    payload: web::Json<FolderCreateRequest>,
    req: HttpRequest,
) -> impl Responder {
    // req.extensions() を一度変数に格納することでライフタイムを管理
    let extensions = req.extensions();

    // JWT から user_id を取得
    let claims = match extensions.get::<Claims>() {
        Some(claims) => claims,
        None => return HttpResponse::Unauthorized().body("認証情報が見つかりません"),
    };

    // claimsを一時変数に束縛し、後で user_id を取得
    let user_id = match claims.sub.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return HttpResponse::Unauthorized().body("無効なユーザーID"),
    };

    println!("{:?}", user_id);

    let result = sqlx::query!(
        "
        INSERT INTO folders (user_id, name, description, parent_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        ",
        Some(user_id),
        payload.name,
        payload.description,
        payload.parent_id,
    )
    .fetch_one(db_pool.get_ref())
    .await;

    match result {
        Ok(record) => HttpResponse::Ok().json({
            serde_json::json!({
                "message": "フォルダ作成成功",
                "id": record.id
            })
        }),
        Err(e) => {
            eprintln!("フォルダ作成エラー: {:?}", e);
            HttpResponse::InternalServerError().body("フォルダ作成に失敗しました")
        }
    }
}
