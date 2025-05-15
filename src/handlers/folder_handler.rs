use actix_web::{delete, patch, post, web::{self}, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use crate::{handlers::auth_handler::extract_user_from_jwt, models::folder::{FolderDeleteRequest, FolderUpdateRequest}};
use crate::message;

#[derive(Debug, Deserialize)]
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

#[delete("/delete-folder")]
pub async fn delete_folder(
    db_pool: web::Data<sqlx::PgPool>,
    payload: web::Json<FolderDeleteRequest>,
    req: HttpRequest,
) -> impl Responder {
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let folder_id = payload.folder_id;

    // DBトランザクション開始
    let mut tx = match db_pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("トランザクション開始エラー: {:?}", e);
            return HttpResponse::InternalServerError().body(message::AppError::InternalServerError.message());
        }
    };

    println!("{:?}", tx);

    let folder_check = sqlx::query_scalar!(
        "SELECT id FROM folders WHERE id = $1 AND user_id = $2",
        folder_id,
        claims.user_id,
    )
    .fetch_optional(&mut *tx)
    .await;

    if let Err(e) = folder_check {
        eprintln!("フォルダ確認失敗: {:?}", e);
        return HttpResponse::InternalServerError().body(message::AppError::InternalServerError.message());
    }

    if folder_check.unwrap().is_none() {
        return HttpResponse::NotFound().body("フォルダが存在しないか、権限がありません");
    }

    // 関連するphoto取得
    let photos = match sqlx::query!(
        "SELECT image_path FROM photos WHERE folder_id = $1",
        folder_id
    )
    .fetch_all(&mut *tx)
    .await
    {
        Ok(photos) => photos,
        Err(e) => {
            eprintln!("写真取得エラー: {:?}", e);
            return HttpResponse::InternalServerError().body(message::AppError::InternalServerError.message());
        }
    };

    if photos.is_empty() {
        match sqlx::query!(
            "DELETE FROM folders WHERE id = $1 AND user_id = $2",
            folder_id,
            claims.user_id,
        )
        .execute(&mut *tx)
        .await
        {
            Ok(_) => {
                if let Err(e) = tx.commit().await {
                    eprintln!("コミット失敗: {:?}", e);
                    return HttpResponse::InternalServerError().body(message::AppError::InternalServerError.message());
                }

                HttpResponse::Ok().json(serde_json::json!({ "message": "フォルダ削除成功" }))
            }
            Err(e) => {
                eprintln!("フォルダ削除失敗: {:?}", e);
                HttpResponse::InternalServerError().body(message::AppError::DeleteFailed(message::FileType::Folder).message())
            }
        }
    } else {
        eprintln!("フォルダ内に {} 件の写真が存在します。削除できません。", photos.len());
        HttpResponse::BadRequest().body("フォルダ内に写真が存在するため削除できません")
    }
}

#[patch("/update-folder")]
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
            "message": message::AppSuccess::UpdatedFolder("folder".to_string()).message(),
            "folder_id": record.id,
            "new_name": record.name,
            "new_description": record.description
        })),
        Err(e) => {
            eprintln!("フォルダ更新エラー: {:?}", e);
            HttpResponse::InternalServerError().body("フォルダの更新に失敗しました")
        }
    }
}

// // S3削除
// let (s3_client, bucket_name, _) = create_s3_client();
// for photo in &photos {
//     if let Some(s3_key) = &photo.s3_key {
//         if let Err(e) = s3_client
//             .delete_object()
//             .bucket(&bucket_name)
//             .key(s3_key)
//             .send()
//             .await
//         {
//             eprintln!("S3削除失敗: {:?}", e);
//         }
//     }
// }

// // DBからphotos削除
// if let Err(e) = sqlx::query!(
//     "DELETE FROM photos WHERE folder_id = $1",
//     folder_id
// )
// .execute(&mut *tx)
// .await
// {
//     eprintln!("写真削除失敗: {:?}", e);
//     return HttpResponse::InternalServerError().body("内部エラー");
// }

// // フォルダ削除
// if let Err(e) = sqlx::query!(
//     "DELETE FROM folders WHERE id = $1",
//     folder_id
// )
// .execute(&mut *tx)
// .await
// {
//     eprintln!("フォルダ削除失敗: {:?}", e);
//     return HttpResponse::InternalServerError().body("内部エラー");
// }

// // トランザクションコミット
// if let Err(e) = tx.commit().await {
//     eprintln!("トランザクションコミット失敗: {:?}", e);
//     return HttpResponse::InternalServerError().body("内部エラー");
// }

// HttpResponse::Ok().json(serde_json::json!({
//     "message": "フォルダ削除成功",
//     "deleted_folder_id": folder_id
// }))
