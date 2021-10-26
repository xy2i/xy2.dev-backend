use crate::db::Pool;
use crate::schema::comments;
use actix_cors::Cors;
use actix_web::dev::HttpResponseBuilder;
use actix_web::http::StatusCode;
use actix_web::{
    error, get, http::header::ContentType, post, web, App, Error, HttpRequest, HttpResponse,
    HttpServer, Responder,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::PgConnection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io;
use std::rc::Rc;
use validator::{Validate, ValidationError, ValidationErrorsKind};

#[derive(Insertable, Debug, Validate, Deserialize)]
#[table_name = "comments"]
pub struct NewComment {
    pub slug: String,
    #[validate(length(min = 2, max = 64, message = ""))]
    pub name: String,
    #[validate(length(min = 10, max = 10_000))]
    pub text: String,
    pub email: String,
    pub parent: Option<i32>,
}

#[derive(Debug, Queryable, Serialize, Deserialize)]
struct Comment {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub date: DateTime<Utc>,
    pub parent: Option<i32>,
    pub text: String,
    pub email: Option<String>,
}

impl Comment {
    fn new(pool: &Pool, new_comment: NewComment) -> Result<Comment> {
        let conn = pool.get()?;
        let new = diesel::insert_into(comments::table)
            .values(&new_comment)
            .get_result(&conn)?;
        Ok(new)
    }

    fn fetch_all(pool: &Pool) -> Result<Vec<Comment>> {
        let conn = pool.get()?;
        let comments: Vec<Comment> = comments::table.load::<Comment>(&conn)?;
        Ok(comments)
    }

    fn fetch_slug(pool: &Pool, slug: &str) -> Result<Vec<Comment>> {
        let conn = pool.get()?;
        let comments: Vec<Comment> = comments::table
            .filter(comments::slug.eq(slug))
            .load::<Comment>(&conn)?;
        Ok(comments)
    }

    fn update(pool: &Pool, id: i32, new_text: &str) -> Result<Comment> {
        use crate::schema::comments::dsl::{comments, text};

        let conn = pool.get()?;
        let updated = diesel::update(comments.find(id))
            .set(text.eq(new_text))
            .get_result::<Comment>(&conn)?;
        Ok(updated)
    }
}

#[get("/comments")]
pub async fn get_all_comments(pool: web::Data<Pool>) -> Result<HttpResponse, Error> {
    Ok(web::block(move || Comment::fetch_all(&pool))
        .await
        .map(|comments| HttpResponse::Ok().json(comments))
        .map_err(|_| HttpResponse::InternalServerError())?)
}

#[get("/comments/{slug}")]
pub async fn get_comments_by_slug(
    pool: web::Data<Pool>,
    slug: web::Path<String>,
) -> Result<HttpResponse, Error> {
    Ok(web::block(move || Comment::fetch_slug(&pool, &slug))
        .await
        .map(|comments| HttpResponse::Ok().json(comments))
        .map_err(|_| HttpResponse::InternalServerError())?)
}

#[derive(Debug, Deserialize)]
pub struct CommentReq {
    pub name: String,
    pub text: String,
    pub email: String,
    pub parent: Option<i32>,
}

#[post("/comments/{slug}")]
pub async fn post_comment(
    pool: web::Data<Pool>,
    web::Path(slug): web::Path<String>,
    web::Json(comment_req): web::Json<CommentReq>,
) -> Result<HttpResponse, Error> {
    println!("comment: {:?}", comment_req);
    let comment = NewComment {
        slug,
        name: comment_req.name,
        text: comment_req.text,
        email: comment_req.email,
        parent: comment_req.parent,
    };

    comment.validate().map_err(|e| {
        HttpResponse::BadRequest().body(format!(
            "{:?}",
            e.into_errors().into_values().collect::<Vec<_>>()
        ))
    })?;

    Ok(web::block(move || Comment::new(&pool, comment))
        .await
        .map(|comments| HttpResponse::Ok().json(comments))
        .map_err(|_| HttpResponse::InternalServerError())?)
}
