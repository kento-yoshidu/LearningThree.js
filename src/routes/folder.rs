use actix_web::web;

use crate::handlers::folder_handler::get_folder_contents;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(get_folder_contents);
}
