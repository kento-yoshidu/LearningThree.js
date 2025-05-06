use actix_web::{delete, post, web, HttpRequest, HttpResponse, Responder};
use crate::handlers::auth_handler::extract_user_from_jwt;
use super::files_handler::PhotoCreateRequest;

#[post("/register-photo")]
pub async fn register_photo(
    req: HttpRequest,
    db_pool: web::Data<sqlx::PgPool>,
    payload: web::Json<PhotoCreateRequest>,
) -> impl Responder {
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let result = sqlx::query!(
        "
        INSERT INTO photos
            (user_id, title, folder_id, description, image_path)
        VALUES
            ($1, $2, $3, $4, $5)
        ",
        claims.user_id,
        payload.title.as_deref(),
        payload.folder_id,
        payload.description.as_deref(),
        payload.image_path,
    )
    .execute(db_pool.get_ref())
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().body("保存成功"),
        Err(e) => {
            eprintln!("DB保存エラー: {:?}", e);
            HttpResponse::InternalServerError().body("保存失敗")
        }
    }
}

#[delete("/delete-photo")]
pub async fn delete_photo(
    req: HttpRequest,
    db_pool: web::Data<sqlx::PgPool>,
    photo_ids: web::Json<Vec<i32>>,
) -> impl Responder {
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    if photo_ids.is_empty() {
        return HttpResponse::BadRequest().body("削除対象のIDがありません");
    }

    let result = sqlx::query(
        "
        DELETE FROM photos
        WHERE id = ANY($1) AND user_id = $2
        ",
    )
    .bind(photo_ids.as_slice())
    .bind(claims.user_id)
    .execute(db_pool.get_ref())
    .await;

    match result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                HttpResponse::NotFound().body("対象の写真が見つからない、または削除権限がありません")
            } else {
                HttpResponse::Ok().body("削除成功")
            }
        }
        Err(e) => {
            eprintln!("写真削除エラー: {:?}", e);
            HttpResponse::InternalServerError().body("削除失敗")
        }
    }
}
