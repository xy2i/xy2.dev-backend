use actix_ratelimit::{MemoryStore, MemoryStoreActor, RateLimiter};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use actix_web::dev::HttpServiceFactory;
use actix_web::{get, post, web, Error, HttpResponse, HttpResponseBuilder, Responder, Scope};
use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{debug, error, info, log_enabled, Level};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgRow;
use sqlx::PgPool;
use validator::Validate;

/// A comment as represented in the database.
#[derive(Debug, Serialize, Deserialize)]
pub struct Comment {
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
    pub email: String,
    pub parent: Option<i32>,
}

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
    pub children: Mutex<JsonCommentTree>,
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
    fn make_tree(comments: Vec<Comment>) -> JsonCommentTree {
        let parents: Vec<Option<i32>> = comments.iter().map(|c| c.parent).collect();
        let nodes: Vec<Arc<JsonComment>> = comments
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
    //     fn new(pool: &PgPool, new_comment: NewComment) -> Result<Self> {
    //         let conn = pool.get()?;
    //         let new = diesel::insert_into(comments::table)
    //             .values(&new_comment)
    //             .get_result(&conn)?;
    //         Ok(new)
    //     }

    async fn fetch_all(pool: &PgPool) -> Result<Vec<Comment>> {
        let res = sqlx::query_as!(Comment, "select * from comments")
            .fetch_all(pool)
            .await?;
        Ok(res)
    }

    // fn fetch_slug(pool: &Pool, slug: &str) -> Result<JsonCommentTree> {
    //     let conn = pool.get()?;
    //     let comments: Vec<Self> = comments::table
    //         .filter(comments::slug.eq(slug))
    //         .load(&conn)?;
    //     Ok(JsonComment::make_tree(comments))
    // }
    //
    // fn update(pool: &Pool, id: i32, new_text: &str) -> Result<Self> {
    //     use crate::schema::comments::dsl::{comments, text};
    //
    //     let conn = pool.get()?;
    //     let updated = diesel::update(comments.find(id))
    //         .set(text.eq(new_text))
    //         .get_result(&conn)?;
    //     Ok(updated)
    // }
}

#[get("/{slug}")]
async fn get_comments_by_slug(
    pool: web::Data<PgPool>, /*slug: web::Path<String>*/
) -> HttpResponse {
    Comment::fetch_all(&pool).await.map_or_else(
        |e| HttpResponse::InternalServerError().body(e.to_string()),
        |res| {
            debug!("{:?}", res);
            HttpResponse::Ok().json(res)
        },
    )
}
//
// /// A request to create a comment.
// #[derive(Debug, Deserialize)]
// struct CommentReq {
//     pub name: String,
//     pub text: String,
//     pub email: String,
//     pub parent: Option<i32>,
// }
//
// #[post("/{slug}")]
// async fn post_comment(
//     pool: web::Data<Pool>,
//     web::Path(slug): web::Path<String>,
//     web::Json(comment_req): web::Json<CommentReq>,
// ) -> impl Responder {
//     let comment = NewComment {
//         slug,
//         name: comment_req.name,
//         text: comment_req.text,
//         email: comment_req.email,
//         parent: comment_req.parent,
//     };
//
//     comment.validate().map_err(|e| {
//         HttpResponse::BadRequest().body(format!(
//             "{:?}",
//             e.into_errors().into_values().collect::<Vec<_>>()
//         ))
//     })?;
//
//     web::block(move || Comment::new(&pool, comment))
//         .await
//         .map(|comments| HttpResponse::Ok().json(comments))
//         .map_err(|_| HttpResponse::InternalServerError())
// }

impl Comment {
    pub fn configure(cfg: &mut web::ServiceConfig) {
        let store = MemoryStore::new();

        cfg.service(
            web::scope("comments")
                .service(get_comments_by_slug)
            // .service(
            //     web::scope("")
            //         .wrap(
            //             RateLimiter::new(MemoryStoreActor::from(store.clone()).start())
            //                 .with_interval(Duration::from_secs(120))
            //                 .with_max_requests(5),
            //         )
            //         .service(post_comment),
            // ),
        );
    }
}
