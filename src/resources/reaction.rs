//! A reaction to a post.
use actix_http::Response;
use actix_service::ServiceFactory;
use actix_web::dev::HttpServiceFactory;
use actix_web::error::BlockingError;
use actix_web::{get, web, Error, HttpResponse, Responder, Scope};
use anyhow::Result;
use log::error;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Duration;

/// A reaction to a post, characterized by the slug.
#[derive(Debug, Serialize)]
struct Reaction {
    pub upvotes: i32,
}

impl Reaction {
    async fn fetch_slug(pool: &PgPool, slug: &str) -> Result<Option<Self>> {
        let res = sqlx::query_as!(
            Reaction,
            r#"
            select upvotes from reaction
            where slug = $1
            "#,
            slug
        )
        .fetch_optional(pool)
        .await?;
        Ok(res)
    }

    async fn upvote_post(pool: &PgPool, slug: &str) -> Result<Self> {
        let res = sqlx::query_as!(
            Reaction,
            r#"
            update reaction
            set upvotes = upvotes + 1
            where slug = $1
            returning upvotes
            "#,
            slug
        )
        .fetch_one(pool)
        .await?;
        Ok(res)
    }
}

#[get("/{slug}")]
async fn get_reaction(pool: web::Data<PgPool>, slug: web::Path<String>) -> impl Responder {
    Reaction::fetch_slug(&pool, &slug)
        .await
        .map_err(|_| HttpResponse::InternalServerError().finish())
        .map(|reaction| {
            reaction.map_or_else(
                || HttpResponse::BadRequest().finish(),
                |v| HttpResponse::Ok().json(v),
            )
        })
}

#[get("/{slug}/upvote")]
async fn upvote_post(pool: web::Data<PgPool>, slug: web::Path<String>) -> impl Responder {
    Reaction::upvote_post(&pool, &slug)
        .await
        .map(|v| HttpResponse::Ok().json(v))
        .map_err(|_| HttpResponse::InternalServerError())
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("reaction")
            .service(get_reaction)
            .service(upvote_post),
    );
}
