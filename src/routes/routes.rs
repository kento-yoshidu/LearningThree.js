use actix_web::web;

use crate::handlers::user_handler::{signup, signin};
use crate::handlers::files_handler::{get_folder_contents, register_photo};
use crate::handlers::folder_handler::create_folder;
use crate::handlers::tags_handler::get_tags;
use crate::handlers::generate_presigned_url::generate_presigned_url;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
        // .service(signup)
        // .service(signin)
        .service(get_folder_contents)
        .service(register_photo)
        .service(create_folder)
        .service(get_tags)
        .service(generate_presigned_url);
}
