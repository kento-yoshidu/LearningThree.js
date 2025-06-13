use std::collections::HashMap;

use actix_web::{get, delete, post, put, web, HttpRequest, HttpResponse, Responder};
use serde::Serialize;
use crate::{handlers::{auth_handler::extract_user_from_jwt, s3_handler::delete_image_from_s3}, models::{photo::{PhotoDeleteRequest, PhotoMoveRequest, PhotoResponse, PhotoSearchRequest, PhotoUpdateRequest, PhotoUploadRequest, PhotoWrapper, TagAddRequest}, tag::AddTagRequest, Tag}, utils::s3::create_s3_client};
use crate::message;

#[derive(Debug, Serialize)]
struct PhotoWithTags {
    id: i32,
    tags: Vec<Tag>,
}

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
                    width: row.width,
                    height: row.height,
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
            (
                user_id,
                name,
                folder_id,
                description,
                image_path,
                size_in_bytes)
        VALUES
            ($1, $2, $3, $4, $5, $6)
        ",
        claims.user_id,
        payload.name.as_deref(),
        payload.folder_id,
        payload.description.as_deref(),
        payload.image_path,
        payload.size_in_bytes,
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

#[post("/photos/tags")]
pub async fn add_tag_to_photo(
    req: HttpRequest,
    db_pool: web::Data<sqlx::PgPool>,
    payload: web::Json<TagAddRequest>,
) -> impl Responder {
    println!("発火");
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let mut tx = match db_pool.begin().await {
        Ok(t) => t,
        Err(_) => {
            eprintln!("トランザクション開始失敗");
            return HttpResponse::InternalServerError().finish();
        }
    };

    // タグの所有者チェック（タグが自分のものか）
    let tag_rows = match sqlx::query!(
        "SELECT id FROM tags WHERE user_id = $1 AND id = ANY($2)",
        claims.user_id,
        &payload.tag_ids
    )
    .fetch_all(&mut *tx)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("タグ所有権チェック失敗: {:?}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let owned_tag_ids: Vec<i32> = tag_rows.into_iter().map(|r| r.id).collect();

    for tag_id in &payload.tag_ids {
        if !owned_tag_ids.contains(tag_id) {
            return HttpResponse::Forbidden().json(serde_json::json!({
                "message": format!("タグID {} はあなたのタグではありません", tag_id)
            }));
        }
    }

    // 写真の所有者チェック
    let photo_rows = match sqlx::query!(
        "SELECT id FROM photos WHERE user_id = $1 AND id = ANY($2)",
        claims.user_id,
        &payload.photo_ids
    )
    .fetch_all(&mut *tx)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("写真所有権チェック失敗: {:?}", e);
            return HttpResponse::InternalServerError().finish();
        }
    };

    let owned_photo_ids: Vec<i32> = photo_rows.into_iter().map(|r| r.id).collect();

    for photo_id in &payload.photo_ids {
        if !owned_photo_ids.contains(photo_id) {
            return HttpResponse::Forbidden().json(serde_json::json!({
                "message": format!("写真ID {} はあなたの写真ではありません", photo_id)
            }));
        }
    }

    for photo_id in &payload.photo_ids {
        // 既存タグを削除
        if let Err(e) = sqlx::query!(
            "DELETE FROM photo_tag_relations WHERE photo_id = $1",
            photo_id
        )
        .execute(&mut *tx)
        .await
        {
            eprintln!("photo_id {} の既存タグ削除失敗: {:?}", photo_id, e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "既存のタグ削除に失敗しました"
            }));
        }

        // 新しいタグを追加
        for tag_id in &payload.tag_ids {
            if let Err(e) = sqlx::query!(
                "INSERT INTO photo_tag_relations (photo_id, tag_id)
                    VALUES ($1, $2)
                    ON CONFLICT DO NOTHING",
                photo_id,
                tag_id
            )
            .execute(&mut *tx)
            .await
            {
                eprintln!("photo_id {} に tag_id {} を追加中にエラー: {:?}", photo_id, tag_id, e);
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "message": "タグの追加に失敗しました"
                }));
            }
        }
    }

    // タグ情報を取得して返す
    let rows = match sqlx::query!(
        r#"
        SELECT ptr.photo_id, t.id AS tag_id, t.tag
        FROM photo_tag_relations ptr
        JOIN tags t ON ptr.tag_id = t.id
        WHERE ptr.photo_id = ANY($1)
        "#,
        &payload.photo_ids
    )
    .fetch_all(db_pool.get_ref())
    .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("タグ取得失敗: {:?}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "タグ情報の取得に失敗しました"
            }));
        }
    };

    let mut map: HashMap<i32, Vec<Tag>> = HashMap::new();

    for row in rows {
        map.entry(row.photo_id)
            .or_insert_with(Vec::new)
            .push(Tag {
                id: row.tag_id,
                user_id: Some(claims.user_id),
                tag: row.tag,
            });
    }

    let updated_photos: Vec<PhotoWithTags> = map
        .into_iter()
        .map(|(id, tags)| PhotoWithTags { id, tags })
        .collect();

    // トランザクションの最後に追加
    if let Err(e) = tx.commit().await {
        eprintln!("トランザクションのコミットに失敗: {:?}", e);
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().json(serde_json::json!({
        "message": "タグを更新しました",
        "updated_photos": updated_photos
    }))
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
