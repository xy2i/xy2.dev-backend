// use crate::db::Pool;
// use crate::schema::reaction;
// use actix_ratelimit::{MemoryStore, MemoryStoreActor, RateLimiter};
// use actix_service::ServiceFactory;
// use actix_web::dev::HttpServiceFactory;
// use actix_web::error::BlockingError;
// use actix_web::{get, web, Error, HttpResponse, Responder, Scope};
// use anyhow::Result;
// use diesel::prelude::*;
// use serde::{Deserialize, Serialize};
// use std::time::Duration;
//
// /// A reaction to a post, characterized by the slug.
// #[derive(Debug, Queryable, Serialize)]
// pub struct Reaction {
//     pub slug: String,
//     pub upvotes: i32,
// }
//
// impl Reaction {
//     fn fetch_slug(pool: &Pool, slug: &str) -> Result<Self> {
//         let conn = pool.get()?;
//         let res = reaction::table
//             .filter(reaction::slug.eq(slug))
//             .first(&conn)?;
//
//         Ok(res)
//     }
//
//     fn upvote_post(pool: &Pool, slug: &str) -> Result<Self> {
//         use crate::schema::reaction::dsl::{reaction, upvotes};
//
//         let conn = pool.get()?;
//
//         let res = diesel::update(reaction.find(slug))
//             .set(upvotes.eq(upvotes + 1))
//             .get_result(&conn)?;
//
//         Ok(res)
//     }
// }
//
// #[get("/{slug}")]
// async fn get_reaction(pool: web::Data<Pool>, slug: web::Path<String>) -> impl Responder {
//     web::block(move || Reaction::fetch_slug(&pool, &slug))
//         .await
//         .map(|v| HttpResponse::Ok().json(v))
//         .map_err(|_| HttpResponse::InternalServerError())
// }
//
// #[get("/{slug}/upvote")]
// async fn upvote_post(pool: web::Data<Pool>, slug: web::Path<String>) -> impl Responder {
//     web::block(move || Reaction::upvote_post(&pool, &slug))
//         .await
//         .map(|v| HttpResponse::Ok().json(v))
//         .map_err(|_| HttpResponse::InternalServerError())
// }
//
// impl Reaction {
//     pub fn configure(cfg: &mut web::ServiceConfig) {
//         let store = MemoryStore::new();
//         cfg.service(
//             web::scope("reaction").service(get_reaction).service(
//                 web::scope("")
//                     .wrap(
//                         RateLimiter::new(MemoryStoreActor::from(store.clone()).start())
//                             .with_interval(Duration::from_secs(60))
//                             .with_max_requests(5),
//                     )
//                     .service(upvote_post),
//             ),
//         );
//     }
// }
