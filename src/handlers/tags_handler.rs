use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use sqlx::PgPool;
use crate::{handlers::auth_handler::extract_user_from_jwt, models::Tag};

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
