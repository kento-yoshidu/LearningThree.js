use actix_web::web;

use crate::handlers::files_handler::get_folder_contents;
use crate::handlers::tags_handler::get_tags;
use crate::handlers::generate_presigned_url::generate_presigned_url;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
        .service(get_folder_contents)
        .service(get_tags)
        .service(generate_presigned_url);
}
