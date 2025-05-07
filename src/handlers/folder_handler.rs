use actix_web::{delete, post, web::{self, Payload}, HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use crate::{handlers::auth_handler::extract_user_from_jwt, models::folder::FolderDeleteRequest};

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
            return HttpResponse::InternalServerError().body("内部エラー");
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
        return HttpResponse::InternalServerError().body("内部エラー");
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
            return HttpResponse::InternalServerError().body("内部エラー");
        }
    };

    println!("{:?}", photos);

    if photos.is_empty() {
        // フォルダーを削除
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
                    return HttpResponse::InternalServerError().body("内部エラー");
                }

                HttpResponse::Ok().json(serde_json::json!({ "message": "フォルダ削除成功" }))
            }
            Err(e) => {
                eprintln!("フォルダ削除失敗: {:?}", e);
                HttpResponse::InternalServerError().body("フォルダ削除に失敗しました")
            }
        }
    } else {
        eprintln!("フォルダ内に {} 件の写真が存在します。削除できません。", photos.len());
        HttpResponse::BadRequest().body("フォルダ内に写真が存在するため削除できません")
    }
}
