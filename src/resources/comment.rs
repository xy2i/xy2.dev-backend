use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use actix_web::{get, post, web, HttpResponse, Responder};
use anyhow::Result;
use chrono::{DateTime, Utc};
use log::error;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use validator::Validate;

/// A comment as represented in the database.
#[derive(Debug, Serialize, Deserialize)]
struct Comment {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub date: DateTime<Utc>,
    pub parent: Option<i32>,
    pub text: String,
    pub email: Option<String>,
    /// If false, comment is in moderation queue
    pub visible: bool,
}

/// A new comment, about to be inserted in the database.
#[derive(Debug, Validate, Deserialize)]
struct NewComment {
    pub slug: String,
    #[validate(length(min = 2, max = 64, message = ""))]
    pub name: String,
    #[validate(length(min = 10, max = 10_000))]
    pub text: String,
    pub email: Option<String>,
    pub parent: Option<i32>,
}

type JsonCommentTree = Vec<Arc<JsonComment>>;

/// A comment that has references to its children directly. Useful
/// for being easy to consume for the frontend
#[derive(Debug, Serialize)]
struct JsonComment {
    pub id: i32,
    pub name: String,
    pub date: DateTime<Utc>,
    pub text: String,
    pub email: Option<String>,
    /// Children of this comment
    pub children: Mutex<JsonCommentTree>,
}

impl JsonComment {
    fn new(comment: Comment) -> JsonComment {
        JsonComment {
            id: comment.id,
            name: comment.name,
            date: comment.date,
            text: comment.text,
            email: comment.email,
            children: Mutex::new(vec![]),
        }
    }

    fn add_child(node: &Arc<JsonComment>, child: &Arc<JsonComment>) {
        node.children.lock().unwrap().push(Arc::clone(child));
    }

    /// Create a tree of comments from a vector.
    fn make_tree(comments: Vec<Comment>) -> JsonCommentTree {
        let parents: Vec<Option<i32>> = comments.iter().map(|c| c.parent).collect();
        let nodes = comments.into_iter().map(|c| Arc::new(JsonComment::new(c)));

        let mut id_to_node = HashMap::new();

        let mut root_nodes_ids = vec![];

        for (i, node) in nodes.enumerate() {
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
        root_nodes_ids
            .into_iter()
            .map(|root_id| id_to_node.remove(&root_id).unwrap())
            .collect()
    }
}

impl Comment {
    async fn new(pool: &PgPool, new_comment: NewComment) -> Result<Self> {
        let res = sqlx::query_as!(
            Comment,
            r#"
            insert into comments (slug, name, text, email, parent)
            values ($1, $2, $3, $4, $5)
            returning *
            "#,
            new_comment.slug,
            new_comment.name,
            new_comment.text,
            new_comment.email,
            new_comment.parent,
        )
        .fetch_one(pool)
        .await
        .map_err(|e| {
            error!("{}", e);
            e
        })?;
        Ok(res)
    }
    //
    // async fn fetch_all(pool: &PgPool) -> Result<Vec<Comment>> {
    //     let res = sqlx::query_as!(Comment, "select * from comments")
    //         .fetch_all(pool)
    //         .await?;
    //     Ok(res)
    // }

    async fn fetch_slug(pool: &PgPool, slug: &str) -> Result<JsonCommentTree> {
        let res = sqlx::query_as!(
            Comment,
            r#"
            select * from comments
            where slug = $1
            "#,
            slug
        )
        .fetch_all(pool)
        .await?;
        Ok(JsonComment::make_tree(res))
    }
    //
    // async fn update(pool: &PgPool, slug: &str, new_text: &str) -> Result<Self> {
    //     let res = sqlx::query_as!(
    //         Comment,
    //         r#"
    //         update comments
    //         set text = $1 where slug = $2
    //         "#,
    //         new_text,
    //         slug
    //     )
    //     .execute(pool)
    //     .await?;
    //     Ok(res)
    // }
}

#[get("/{slug}")]
async fn get_comments_by_slug(pool: web::Data<PgPool>, slug: web::Path<String>) -> impl Responder {
    Comment::fetch_slug(&pool, &slug)
        .await
        .map(|comment| HttpResponse::Ok().json(comment))
        .map_err(|_| HttpResponse::InternalServerError().finish())
}

/// A request to create a comment.
#[derive(Debug, Deserialize)]
struct CommentReq {
    pub name: String,
    pub text: String,
    pub email: Option<String>,
    pub parent: Option<i32>,
}

#[post("/{slug}")]
async fn post_comment(
    pool: web::Data<PgPool>,
    slug: web::Path<String>,
    web::Json(comment_req): web::Json<CommentReq>,
) -> impl Responder {
    let comment = NewComment {
        slug: slug.into_inner(),
        name: comment_req.name,
        text: comment_req.text,
        email: comment_req.email,
        parent: comment_req.parent,
    };

    comment.validate().map_err(|e| {
        error!("{}", e);
        HttpResponse::BadRequest().finish()
    })?;

    Comment::new(&pool, comment)
        .await
        .map(|comment| HttpResponse::Ok().json(comment))
        .map_err(|_| HttpResponse::InternalServerError())
}

/// Actix configuration for comments.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("comments")
            .service(get_comments_by_slug)
            .service(post_comment),
    );
}
