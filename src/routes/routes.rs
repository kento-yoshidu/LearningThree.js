use actix_web::web;

use crate::handlers::files_handler::{
    get_folder_contents,
    get_all_photos,
};
use crate::handlers::photo_handler::{
    upload_photo,
    update_photo,
    move_photo,
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
        // 写真
        .service(upload_photo)
        .service(update_photo)
        .service(move_photo)
        .service(delete_photo)
        .service(search_photos)
        .service(generate_presigned_url)
        // フォルダー
        .service(create_folder)
        .service(update_folder)
        .service(delete_folder)
        // タグ
        .service(get_tags)
        .service(add_tag);
}
