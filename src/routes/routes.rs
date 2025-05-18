use actix_web::web;

use crate::handlers::files_handler::{
    get_folder_contents,
    get_all_photos,
};
use crate::handlers::photo_handler::{
    upload_photo,
    update_photo,
    delete_photo,
    search_photos,
};
use crate::handlers::folder_handler::{
    create_folder,
    update_folder,
    delete_folder,
};
use crate::handlers::tags_handler::{
    get_tags,
    add_tag,
};

use crate::handlers::generate_presigned_url::generate_presigned_url;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
        .service(get_folder_contents)
        .service(get_all_photos)
        .service(upload_photo)
        .service(update_photo)
        .service(delete_photo)
        .service(search_photos)
        .service(create_folder)
        .service(update_folder)
        .service(delete_folder)
        .service(get_tags)
        .service(add_tag)
        .service(generate_presigned_url);
}
