use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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
use serde_json;
use validator::{Validate, ValidationError, ValidationErrorsKind};

use crate::db::Pool;
use crate::schema::comments;

/// A new comment, about to be inserted in the database.
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

/// A comment as represented in the database.
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

// TODO: Is there a way to refactor this to some common fields? The problem is with Diesel,
// with serde we can #[serde(flatten)]
/// A comment that has references to its children directly. Useful
/// for being easy to consume for the frontend
#[derive(Debug, Serialize)]
struct JsonComment {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub date: DateTime<Utc>,
    pub text: String,
    pub email: Option<String>,
    /// Children of this comment
    pub children: Mutex<Vec<Arc<JsonComment>>>,
}

type JsonCommentTree = Vec<Arc<JsonComment>>;

impl JsonComment {
    fn new(comment: Comment) -> JsonComment {
        JsonComment {
            id: comment.id,
            slug: comment.slug,
            name: comment.name,
            date: comment.date,
            text: comment.text,
            email: comment.email,
            children: Mutex::new(vec![]),
        }
    }

    fn add_child(node: &Arc<JsonComment>, child: &Arc<JsonComment>) {
        node.children.lock().unwrap().push(Arc::clone(&child));
    }

    /// Create a tree of comments from a vector.
    fn make_tree(comments: Vec<Comment>) -> Vec<Arc<JsonComment>> {
        let parents: Vec<Option<i32>> = comments.iter().map(|c| c.parent).collect();
        let mut nodes: Vec<Arc<JsonComment>> = comments
            .into_iter()
            .map(|c| Arc::new(JsonComment::new(c)))
            .collect();

        let mut id_to_node = HashMap::new();

        let mut root_nodes_ids = vec![];

        for (i, node) in nodes.into_iter().enumerate() {
            let id = node.id;
            id_to_node.insert(id, node);

            if let Some(parent) = parents[i] {
                let parent = id_to_node.get(&parent).unwrap();
                let node = id_to_node.get(&id).unwrap();
                JsonComment::add_child(parent, node);
            } else {
                // We can't remove the tree from the id_to_node map yet,
                // since later nodes may be children to this node.
                root_nodes_ids.push(id);
            }
        }

        // Collect the tree roots.
        let mut tree = vec![];
        for root_id in root_nodes_ids {
            let root = id_to_node.remove(&root_id).unwrap();
            tree.push(root);
        }

        tree
    }
}

impl Comment {
    fn new(pool: &Pool, new_comment: NewComment) -> Result<Comment> {
        let conn = pool.get()?;
        let new = diesel::insert_into(comments::table)
            .values(&new_comment)
            .get_result(&conn)?;
        Ok(new)
    }

    fn fetch_all(pool: &Pool) -> Result<JsonCommentTree> {
        let conn = pool.get()?;
        let comments: Vec<Comment> = comments::table.load::<Comment>(&conn)?;
        Ok(JsonComment::make_tree(comments))
    }

    fn fetch_slug(pool: &Pool, slug: &str) -> Result<JsonCommentTree> {
        let conn = pool.get()?;
        let comments: Vec<Comment> = comments::table
            .filter(comments::slug.eq(slug))
            .load::<Comment>(&conn)?;
        Ok(JsonComment::make_tree(comments))
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

/// A request to create a comment.
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::comment::{Comment, JsonComment};
    use crate::db::init_pool;

    #[test]
    fn tree() {
        let pool = init_pool().expect("no pool");
        let comments = Comment::fetch_all(&pool).expect("no comments");
        println!("{}", serde_json::to_string(&comments).unwrap());
    }
}
