use actix_web::{HttpResponse, Responder, post, web};
use serde::Deserialize;

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
) -> impl Responder {
    let result = sqlx::query!(
        "
        INSERT INTO folders (user_id, name, description, parent_id)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        ",
        Some(1), // 仮の user_id
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
