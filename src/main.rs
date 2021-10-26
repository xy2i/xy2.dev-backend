mod comment;
mod db;
mod schema;

#[macro_use]
extern crate diesel;

use crate::comment::{get_all_comments, get_comments_by_slug, post_comment};
use crate::db::init_pool;
use actix_cors::Cors;
use actix_web::{get, http, post, web, App, HttpResponse, HttpServer, Responder};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let pool = init_pool().expect("Could not create pool");

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .data(pool.clone())
            .service(get_all_comments)
            .service(get_comments_by_slug)
            .service(post_comment)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
