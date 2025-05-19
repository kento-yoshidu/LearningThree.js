use actix_web::{get, delete, post, put, web, HttpRequest, HttpResponse, Responder};
use aws_sdk_s3::error::SdkError;
use crate::{handlers::auth_handler::extract_user_from_jwt, models::{photo::{PhotoResponse, PhotoSearchRequest, PhotoUpdateRequest, PhotoWrapper}, Photo}, utils::s3::create_s3_client};
use super::files_handler::PhotoCreateRequest;
use crate::message;
use aws_sdk_s3::error::ProvideErrorMetadata;

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
        r#"
        SELECT DISTINCT p.*
        FROM photos p
        JOIN photo_tag_relations ptr ON p.id = ptr.photo_id
        JOIN tags t ON ptr.tag_id = t.id
        WHERE p.user_id = $1
        AND t.tag = ANY($2)
        "#,
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
                    title: row.title,
                    description: row.description,
                    image_path: row.image_path,
                    folder_id: row.folder_id,
                })
                .collect();

            HttpResponse::Ok().json(PhotoWrapper { data: photos })
        }
        Err(_) => HttpResponse::InternalServerError().body("Error searching photos"),
    }
}

#[post("/photos")]
pub async fn upload_photo(
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
            HttpResponse::InternalServerError().body("保存失敗")
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
        SET title = COALESCE($1, title),
            description = COALESCE($2, description)
        WHERE id = $3 AND user_id = $4
        RETURNING id, title, description
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
                    "name": record.title,
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

#[delete("/photos")]
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

    // トランザクション開始
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

        match delete_result {
            Ok(_) => {
                // 削除成功処理
            }
            Err(e) => {
                if let SdkError::ServiceError(service_error) = &e {
                    let err = &service_error.err();

                let code = err.code().unwrap_or_default();

                if code == "NoSuchKey" {
                    println!("存在しないキーなので無視: {:?}", err);
                } else {
                    delete_errors.push(format!("S3削除失敗: {} ({:?})", key, err));
                }
                } else {
                    delete_errors.push(format!("S3削除失敗: {} ({:?})", key, e));
                }
            }
        }
    }

    // 3. photos テーブルから削除
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
