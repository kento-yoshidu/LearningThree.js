use actix_web::{delete, post, web, HttpRequest, HttpResponse, Responder};
use crate::{handlers::auth_handler::extract_user_from_jwt, utils::s3::create_s3_client};
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
    if photo_ids.is_empty() {
        return HttpResponse::BadRequest().body("削除対象のIDがありません");
    }

    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let rows = sqlx::query!(
        "
        SELECT
            image_path
        FROM
            photos
        WHERE id = ANY($1) AND user_id = $2
        ",
        &photo_ids[..],
        claims.user_id
    )
    .fetch_all(db_pool.get_ref())
    .await;

    let filenames = match rows {
        Ok(rows) => rows.into_iter().map(|row| row.image_path).collect::<Vec<String>>(),
        Err(_) => return HttpResponse::InternalServerError().body("画像情報の取得失敗"),
    };

    let mut delete_errors = Vec::new();

    let (client, bucket_name, _) = create_s3_client();

    for url in filenames {
        let key = match url.rsplit('/').next() {
            Some(k) => k,
            None => {
                delete_errors.push(format!("無効なURL形式: {}", url));
                continue;
            }
        };

        let delete_result = client
            .delete_object()
            .bucket(&bucket_name)
            .key(key)
            .send()
            .await;

        if delete_result.is_err() {
            delete_errors.push(format!("S3削除失敗: {}", key));
        }
    }

    let result = sqlx::query!(
        r#"
        DELETE FROM photos
        WHERE id = ANY($1) AND user_id = $2
        "#,
        &photo_ids[..],
        claims.user_id
    )
    .execute(db_pool.get_ref())
    .await;

    match result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                HttpResponse::NotFound().body("対象の写真が見つからない、または削除権限がありません")
            } else {
                if !delete_errors.is_empty() {
                    HttpResponse::InternalServerError().body(delete_errors.join(", "))
                } else {
                    HttpResponse::Ok().body("削除成功")
                }
            }
        }
        Err(_) => HttpResponse::InternalServerError().body("データベース削除失敗"),
    }
}
