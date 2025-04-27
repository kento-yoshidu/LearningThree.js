use actix_web::{web, get, HttpResponse, Responder};
use sqlx::PgPool;
use crate::models::Tag;

#[get("/tags/{user_id}")]
pub async fn get_tags(path: web::Path<i32>, db: web::Data<PgPool>) -> impl Responder {
    let user_id = path.into_inner();

    let tag_rows = sqlx::query!(
        "SELECT
            *
        FROM
            photo_tags
        WHERE
            user_id = $1
        ",
        user_id,
    )
    .fetch_all(db.get_ref())
    .await;

    match tag_rows {
        Ok(rows) => {
            let tags: Vec<Tag> = rows.into_iter().map(|row| Tag {
                id: row.id,
                photo_id: row.photo_id,
                user_id: row.user_id,
                tag: row.tag,
            }).collect();

            HttpResponse::Ok().json(tags)
        },
        Err(_) => HttpResponse::InternalServerError().body("Error fetching tags"),
    }
}