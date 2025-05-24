use actix_web::{post, put, delete, web::{self}, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use crate::{handlers::{auth_handler::extract_user_from_jwt, s3_handler::delete_image_from_s3}, models::folder::{FolderDeleteRequest, FolderUpdateRequest}, utils::s3::create_s3_client};
use crate::message;

#[derive(Debug, Deserialize)]
pub struct FolderCreateRequest {
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
}

#[post("/folders")]
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
                "message": message::AppSuccess::CreatedFolder.message(),
                "id": record.id
            })
        }),
        Err(e) => {
            eprintln!("フォルダ作成エラー: {:?}", e);
            HttpResponse::InternalServerError().body("")
        }
    }
}

#[put("/folders")]
pub async fn update_folder(
    db_pool: web::Data<sqlx::PgPool>,
    payload: web::Json<FolderUpdateRequest>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let folder_check = sqlx::query_scalar!(
        "SELECT
            id
        FROM
            folders
        WHERE id = $1 AND user_id = $2",
        payload.folder_id,
        claims.user_id,
    )
    .fetch_optional(db_pool.get_ref())
    .await;

    let Some(_) = folder_check.ok().flatten() else {
        return HttpResponse::NotFound().body("フォルダが存在しないか、権限がありません");
    };

    let result = sqlx::query!(
        "
        UPDATE folders
        SET name = $1, description = $2
        WHERE id = $3 AND user_id = $4
        RETURNING id, name, description
        ",
        payload.name,
        payload.description,
        payload.folder_id,
        claims.user_id
    )
    .fetch_one(db_pool.get_ref())
    .await;

    match result {
        Ok(record) => HttpResponse::Ok().json(serde_json::json!({
            "message": message::AppSuccess::Updated(message::FileType::Folder).message(),
            "data": {
                "id": record.id,
                "name": record.name,
                "description": record.description,
            }
        })),
        Err(e) => {
            eprintln!("フォルダ更新エラー: {:?}", e);
            HttpResponse::InternalServerError().body("フォルダの更新に失敗しました")
        }
    }
}

#[delete("/folders")]
pub async fn delete_folder(
    db_pool: web::Data<sqlx::PgPool>,
    payload: web::Json<FolderDeleteRequest>,
    req: HttpRequest,
) -> impl Responder {
    let (s3_client, bucket_name, _) = create_s3_client();

    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let folder_ids = &payload.ids;

    let mut tx = match db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("トランザクション開始エラー: {:?}", e);
            return HttpResponse::InternalServerError().body(message::AppError::TransactionStartFailed.message());
        }
    };

    for &folder_id in folder_ids {
        let folder_check = sqlx::query_scalar!(
            "SELECT
                id
            FROM
                folders
            WHERE id = $1 AND user_id = $2",
            folder_id,
            claims.user_id,
        )
        .fetch_optional(&mut *tx)
        .await;

        match folder_check {
            Ok(Some(_)) => {}
            Ok(None) => {
                return HttpResponse::NotFound()
                    .body(format!("フォルダID {} が存在しないか、権限がありません", folder_id));
            }
            Err(e) => {
                eprintln!("フォルダ確認失敗: {:?}", e);
                return HttpResponse::InternalServerError()
                    .body(message::AppError::InternalServerError.message());
            }
        }

        let photos = match sqlx::query!(
            "SELECT
                id,
                image_path
            FROM
                photos
            WHERE folder_id = $1",
            folder_id
        )
        .fetch_all(&mut *tx)
        .await
        {
            Ok(p) => p,
            Err(e) => {
                eprintln!("写真取得失敗: {:?}", e);
                return HttpResponse::InternalServerError().body(message::AppError::InternalServerError.message());
            }
        };

        for photo in photos {
            if let Err(e) = delete_image_from_s3(&s3_client, &bucket_name, &photo.image_path).await {
                eprintln!("S3画像削除失敗: {}", e);
                return HttpResponse::InternalServerError()
                    .body(format!("S3画像削除失敗: {}", e));
            }

            let result = sqlx::query!("DELETE FROM photos WHERE id = $1", photo.id)
                .execute(&mut *tx)
                .await;

            if let Err(e) = result {
                eprintln!("DBからの画像削除失敗: {:?}", e);
                return HttpResponse::InternalServerError().body("写真の削除に失敗しました");
            }
        }

        let delete_result = sqlx::query!(
            "DELETE FROM folders WHERE id = $1 AND user_id = $2",
            folder_id,
            claims.user_id,
        )
        .execute(&mut *tx)
        .await;

        if let Err(e) = delete_result {
            eprintln!("フォルダ削除失敗: {:?}", e);
            return HttpResponse::InternalServerError()
                .body(message::AppError::DeleteFailed(message::FileType::Folder).message());
        }
    }

    if let Err(e) = tx.commit().await {
        eprintln!("トランザクションコミット失敗: {:?}", e);
        return HttpResponse::InternalServerError().body(message::AppError::InternalServerError.message());
    }

    HttpResponse::Ok().json(serde_json::json!({ "message": "フォルダーを削除しました" }))
}
