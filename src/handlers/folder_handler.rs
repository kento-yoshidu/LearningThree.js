use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use crate::handlers::auth_handler::extract_user_from_jwt;

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
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let result = sqlx::query!(
        "
        INSERT INTO folders
            (user_id, name, description, parent_id)
        VALUES
            ($1, $2, $3, $4)
        RETURNING
            id
        ",
        claims.user_id,
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
