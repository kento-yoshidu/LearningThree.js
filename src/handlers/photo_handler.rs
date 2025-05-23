use actix_web::{get, delete, post, put, web, HttpRequest, HttpResponse, Responder};
use crate::{handlers::{auth_handler::extract_user_from_jwt, s3_handler::delete_image_from_s3}, models::{photo::{PhotoDeleteRequest, PhotoMoveRequest, PhotoResponse, PhotoSearchRequest, PhotoUpdateRequest, PhotoUploadRequest, PhotoWrapper}}, utils::s3::create_s3_client};
use crate::message;

#[get("/photos/search")]
pub async fn search_photos(
    req: HttpRequest,
    db_pool: web::Data<sqlx::PgPool>,
    payload: web::Json<PhotoSearchRequest>,
) -> impl Responder {
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let tag_list: Vec<String> = payload
        .tags
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    let rows = sqlx::query!(
        "
        SELECT DISTINCT p.*
        FROM photos p
        JOIN photo_tag_relations ptr ON p.id = ptr.photo_id
        JOIN tags t ON ptr.tag_id = t.id
        WHERE p.user_id = $1
        AND t.tag = ANY($2)
        ",
        claims.user_id,
        &tag_list
    )
    .fetch_all(db_pool.get_ref())
    .await;

    match rows {
        Ok(rows) => {
            let photos: Vec<PhotoResponse> = rows
                .into_iter()
                .map(|row| PhotoResponse {
                    id: row.id,
                    name: row.name,
                    description: row.description,
                    image_path: row.image_path,
                    folder_id: row.folder_id,
                })
                .collect();

            HttpResponse::Ok().json(PhotoWrapper { data: photos })
        }
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "検索に失敗しました。",
        })),
    }
}

#[post("/photos")]
pub async fn upload_photo(
    req: HttpRequest,
    db_pool: web::Data<sqlx::PgPool>,
    payload: web::Json<PhotoUploadRequest>,
) -> impl Responder {
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let result = sqlx::query!(
        "
        INSERT INTO photos
            (user_id, name, folder_id, description, image_path)
        VALUES
            ($1, $2, $3, $4, $5)
        ",
        claims.user_id,
        payload.name.as_deref(),
        payload.folder_id,
        payload.description.as_deref(),
        payload.image_path,
    )
    .execute(db_pool.get_ref())
    .await;

    match result {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": message::AppSuccess::UploadedPhoto.message(),
        })),
        Err(e) => {
            println!("error: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "写真のアップロードに失敗しました。"
            }))
        }
    }
}

#[put("/photos")]
pub async fn update_photo(
    req: HttpRequest,
    db_pool: web::Data<sqlx::PgPool>,
    payload: web::Json<PhotoUpdateRequest>
) -> impl Responder {
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let result = sqlx::query!(
        "
        UPDATE photos
        SET name = COALESCE($1, name),
            description = COALESCE($2, description)
        WHERE id = $3 AND user_id = $4
        RETURNING id, name, description
        ",
        payload.name.as_deref(),
        payload.description.as_deref(),
        payload.id,
        claims.user_id,
    )
    .fetch_optional(db_pool.get_ref())
    .await;

    match result {
        Ok(Some(record)) => {
            HttpResponse::Ok().json(serde_json::json!({
                "message": message::AppSuccess::Updated(message::FileType::Photo).message(),
                "data": {
                    "id": record.id,
                    "name": record.name,
                    "description": record.description,
                },
            }))
        }
        Ok(None) => {
            HttpResponse::NotFound().json(serde_json::json!({
                "message": "写真が見つからない、または更新権限がありません"
            }))
        }
        Err(e) => {
            eprintln!("DB更新エラー: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "更新失敗"
            }))
        }
    }
}

#[put("/photos/move")]
pub async fn move_photo(
    req: HttpRequest,
    db_pool: web::Data<sqlx::PgPool>,
    payload: web::Json<PhotoMoveRequest>,
) -> impl Responder {
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    if payload.ids.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "移動する写真IDが指定されていません"
        }));
    }

    let result = sqlx::query!(
        "
        UPDATE photos
        SET folder_id = $1
        WHERE id = ANY($2) AND user_id = $3
        ",
        payload.folder_id,
        &payload.ids,
        claims.user_id,
    )
    .execute(db_pool.get_ref())
    .await;

    match result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                HttpResponse::NotFound().json(serde_json::json!({
                    "message": "対象の写真が見つからない、または移動権限がありません"
                }))
            } else {
                HttpResponse::Ok().json(serde_json::json!({
                    "message": format!("{}枚の写真を移動しました", res.rows_affected()),
                }))
            }
        }
        Err(e) => {
            eprintln!("フォルダー移動失敗: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "フォルダー移動中にエラーが発生しました"
            }))
        }
    }
}

#[delete("/photos")]
pub async fn delete_photo(
    req: HttpRequest,
    db_pool: web::Data<sqlx::PgPool>,
    payload: web::Json<PhotoDeleteRequest>,
) -> impl Responder {
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let photo_ids = &payload.ids;

    if photo_ids.is_empty() {
        return HttpResponse::BadRequest().body("削除対象のIDがありません");
    }

    let mut tx = match db_pool.begin().await {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().body(message::AppError::TransactionStartFailed.message()),
    };

    // S3削除対象の画像URLを取得
    let rows = sqlx::query!(
        "
        SELECT image_path
        FROM photos
        WHERE id = ANY($1) AND user_id = $2
        ",
        &photo_ids[..],
        claims.user_id
    )
    .fetch_all(&mut *tx)
    .await;

    let filenames = match rows {
        Ok(rows) => rows.into_iter().map(|row| row.image_path).collect::<Vec<String>>(),
        Err(_) => return HttpResponse::InternalServerError().body("画像情報の取得失敗"),
    };

    // 中間テーブルのレコード削除
    let delete_relations_result = sqlx::query!(
        "
        DELETE FROM photo_tag_relations
        WHERE photo_id = ANY($1)
        ",
        &photo_ids[..],
    )
    .execute(&mut *tx)
    .await;

    if let Err(_) = delete_relations_result {
        return HttpResponse::InternalServerError().body("タグ関連データの削除に失敗しました");
    }

    // S3画像削除
    let mut delete_errors = Vec::new();

    let (client, bucket_name, _) = create_s3_client();

    for url in filenames {
        if let Err(e) = delete_image_from_s3(&client, &bucket_name, &url).await {
            delete_errors.push(e);
        }
    }

    // photos テーブルから削除
    let result = sqlx::query!(
        "
        DELETE FROM photos
        WHERE id = ANY($1) AND user_id = $2
        ",
        &photo_ids[..],
        claims.user_id
    )
    .execute(&mut *tx)
    .await;

    match tx.commit().await {
        Ok(_) => (),
        Err(e) => {
            println!("トランザクションコミット失敗: {:?}", e);
            return HttpResponse::InternalServerError().body("トランザクションコミット失敗");
        },
    }

    match result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                HttpResponse::NotFound().body("対象の写真が見つからない、または削除権限がありません")
            } else {
                if !delete_errors.is_empty() {
                    println!("{:?}", delete_errors);

                    HttpResponse::InternalServerError().body(delete_errors.join(", "))
                } else {
                    HttpResponse::Ok().json(serde_json::json!({
                        "message": message::AppSuccess::Deleted(message::FileType::Photo).message(),
                    }))
                }
            }
        },
        Err(e) => {
            println!("{:?}", e);
            HttpResponse::InternalServerError().body("データベース削除失敗")
        },
    }
}
