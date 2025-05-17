use actix_web::{get, post, web::{self, Payload}, HttpRequest, HttpResponse, Responder};
use sqlx::{Executor, PgPool, Postgres, Transaction};
use crate::{handlers::auth_handler::extract_user_from_jwt, message::AppError, models::{tag::{AddTagRequest, TagResponse}, Tag}};

#[get("/tags")]
pub async fn get_tags(
    req: HttpRequest,
    db: web::Data<PgPool>,
) -> impl Responder {
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let tag_rows = sqlx::query!(
        "SELECT
            *
        FROM
            tags
        WHERE
            user_id = $1
        ",
        claims.user_id,
    )
    .fetch_all(db.get_ref())
    .await;

    match tag_rows {
        Ok(rows) => {
            let tags: Vec<Tag> = rows.into_iter().map(|row| Tag {
                id: row.id,
                user_id: Some(row.user_id),
                tag: row.tag,
            }).collect();

            HttpResponse::Ok().json(tags)
        },
        Err(_) => HttpResponse::InternalServerError().body("Error fetching tags"),
    }
}

#[post("/tags")]
pub async fn add_tag(
    req: HttpRequest,
    db: web::Data<PgPool>,
    payload: web::Json<AddTagRequest>,
) -> impl Responder {
    let claims = match extract_user_from_jwt(&req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };

    let mut tx: Transaction<'_, Postgres> = match db.begin().await {
        Ok(tx) => tx,
        Err(_) => return HttpResponse::InternalServerError().body(AppError::TransactionStartFailed.message()),
    };

    let tag_row = sqlx::query!(
        "
            INSERT INTO tags (tag, user_id)
            VALUES ($1, $2)
            ON CONFLICT (tag, user_id) DO NOTHING
            RETURNING id, tag
        ",
        payload.tag,
        claims.user_id,
    )
    .fetch_optional(&mut *tx)
    .await;

    let tag_id = match tag_row {
        Ok(Some(row)) => row.id,
        Ok(None) => {
            match sqlx::query!(
                "SELECT id FROM tags WHERE tag = $1 AND user_id = $2",
                payload.tag,
                claims.user_id,
            )
            .fetch_one(&mut *tx)
            .await {
                Ok(existing) => existing.id,
                Err(_) => {
                    return HttpResponse::InternalServerError().body("Failed to fetch existing tag");
                }
            }
        }
        Err(_) => return HttpResponse::InternalServerError().body("Failed to insert tag"),
    };

    let relation_result = sqlx::query!(
        "
        INSERT INTO photo_tag_relations (photo_id, tag_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        ",
        payload.photo_id,
        tag_id,
    )
    .execute(&mut *tx)
    .await;

    if relation_result.is_err() {
        return HttpResponse::InternalServerError().body("Failed to insert photo-tag relation");
    }

    if let Err(_) = tx.commit().await {
        return HttpResponse::InternalServerError().body("Failed to commit transaction");
    }

    let tag_response = TagResponse {
        id: tag_id,
        tag: payload.tag.clone(),
    };

    HttpResponse::Ok().json(tag_response)
}
