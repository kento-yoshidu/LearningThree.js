use actix_web::web;

use crate::handlers::folder_handler::get_folder_contents;
use crate::handlers::tags_handler::get_tags;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
        .service(get_folder_contents)
        .service(get_tags);
}
