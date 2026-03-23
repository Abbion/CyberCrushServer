use shared_server_lib::{server_configurator::{ServerConfiguration, ServerType}, server_database, common::ResponseStatus};

use axum::{
    extract::{Json, State},
    response::IntoResponse,
    routing::{get, post},
    Router,
};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use sqlx::{ PgPool, Postgres, Transaction };
use chrono;

#[derive(Debug, Serialize, sqlx::FromRow)]
struct NewsArticleEntry {
    id: i32,
    author: String,
    title: String,
    content: String,
    timestamp: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize)]
struct GetNewsFeedResponse {
    response_status: ResponseStatus,
    articles: Vec<NewsArticleEntry>,
}

#[derive(Debug, Deserialize)]
struct PostNewsArticleRequest {
    token: String,
    title: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct PostNewsArticleResponse {
    response_status: ResponseStatus,
    post_id: i32,
}

#[derive(Debug, Deserialize)]
struct DeleteNewsArticleRequest {
    token: String,
    post_id: i32,
}

use ResponseStatus as DeleteNewsArticleResponse;

#[derive(Debug)]
struct ServerState {
    db_pool: PgPool, //This is thread safe
}

#[tokio::main]
async fn main() {
    let server_configuration = ServerConfiguration::load("../server.conf");
    let db_pool = server_database::connect_to_database(server_configuration.get_postgres_connection_url()).await;
    let server_state = Arc::new(ServerState{ db_pool });

    let socket_addr = server_configuration.get_socket_addr(ServerType::News);
    let listener = TcpListener::bind(socket_addr).await.unwrap();
    println!("News server running at: {}", socket_addr);

    let app = Router::new()
        .route("/hello", get(hello))
        .route("/get_news_feed", get(get_news_feed))
        .route("/post_news_article", post(post_news_article))
        .route("/delete_news_article", post(delete_news_article))
        .with_state(server_state);

    axum::serve(listener, app).await.unwrap();
}

async fn hello() -> &'static str {
    "Hello, cyber crush news server!"
}

//TODO add a timestamp paramteter that returns atricles written after that timestamp
async fn get_news_feed(State(state): State<Arc<ServerState>>) -> impl IntoResponse {
    let news_articles_query = sqlx::query_as::<_, NewsArticleEntry>(
    r#"
        SELECT
            na.id,
            pub.username as author,
            na.title,
            na.content,
            na.timestamp
        FROM news_articles na
        JOIN users pub ON na.user_id = pub.id
        ORDER BY na.timestamp DESC
        LIMIT 75;
    "#)
    .fetch_all(&state.db_pool)
    .await;

    let response = match news_articles_query {
        Ok(articles) => GetNewsFeedResponse{ response_status:ResponseStatus::success(), articles: articles },
        Err(error) => {
            eprintln!("Error: Getting news feed failed. Error: {}", error);
            GetNewsFeedResponse{ response_status: ResponseStatus::fail("News feed failed to query. Server error!".into()), articles: Vec::new() }
        }
    };

    Json(response)
}

async fn post_news_article(State(state): State<Arc<ServerState>>, Json(payload): Json<PostNewsArticleRequest>) -> impl IntoResponse {
    // Retrieve publisher user id
    // TODO add checking for the can_publish flag in user_data
    let user_id_query : Result<Option<i32>, sqlx::Error> = sqlx::query_scalar(
    r#"
        SELECT id
        FROM users
        WHERE user_token = $1
    "#)
    .bind(&payload.token)
    .fetch_optional(&state.db_pool)
    .await;
    
    let user_id = match user_id_query {
        Ok(Some(user_id)) => user_id,
        Ok(None) => {
            return Json(PostNewsArticleResponse{ 
                response_status: ResponseStatus::fail("Post publisher not found".into()),
                post_id: -1 });
        },
        Err(error) => {
            eprintln!("Error: Posting news article failed while getting publisher id for token: {}, Error: {}", payload.token, error);
            return Json(PostNewsArticleResponse{ 
                response_status: ResponseStatus::fail("Posting news article publisher identyfication internal server error!".into()),
                post_id: -1 });
        }
    };

    let mut transaction: Transaction<'_, Postgres> = match state.db_pool.begin().await {
        Ok(transaction) => transaction,
        Err(error) => {
            eprintln!("Error: Posting news failed while starting the transaction. Error: {}", error); 
            return Json(PostNewsArticleResponse{ 
                response_status: ResponseStatus::fail("Posting news internal server Error: 1!".into()),
                post_id: -1 });
        }
    };


    let create_news_article_query = sqlx::query_scalar::<_, i32>(
    r#"
        INSERT INTO news_articles
        (user_id, title, content, timestamp)
        VALUES ($1, $2, $3, NOW())
        RETURNING id
    "#)
    .bind(user_id)
    .bind(payload.title)
    .bind(payload.content)
    .fetch_one(&mut *transaction)
    .await;

    let response = match create_news_article_query {
        Ok(id) => {
            PostNewsArticleResponse{ response_status: ResponseStatus::success(), post_id: id }
        },
        Err(error) => {
            eprintln!("Error: Posting news article failed while inserting. Error: {}", error);
            PostNewsArticleResponse{ 
                response_status: ResponseStatus::fail("Posting news internal server Error: 2!".into()),
                post_id: -1 }
        }
    };

    let commited_transaction = transaction.commit().await;
    if commited_transaction.is_err() {
        eprintln!("Error: Posting news failed while commiting transaction. Error: {}", commited_transaction.unwrap_err());
        return Json(PostNewsArticleResponse{ 
                response_status: ResponseStatus::fail("Posting news internal server Error: 3!".into()),
                post_id: -1 });
    }

    Json(response)
}

async fn delete_news_article(State(state): State<Arc<ServerState>>, Json(payload): Json<DeleteNewsArticleRequest>) -> impl IntoResponse {
    let user_id_query : Result<Option<i32>, sqlx::Error> = sqlx::query_scalar(
    r#"
        SELECT id
        FROM users
        WHERE user_token = $1
    "#)
    .bind(&payload.token)
    .fetch_optional(&state.db_pool)
    .await;
    
    let user_id = match user_id_query {
        Ok(Some(user_id)) => user_id,
        Ok(None) => {
            return Json(DeleteNewsArticleResponse::fail("Post publisher not found".into()));
        },
        Err(error) => {
            eprintln!("Error: Deleting news article failed while getting publisher id for token: {}, Error: {}", payload.token, error);
            return Json(DeleteNewsArticleResponse::fail("Deleting news article publisher identyfication internal server error!".into()));
        }
    };

    let delete_post_query = sqlx::query(
    r#"
        DELETE FROM news_articles
        WHERE id = $1 and user_id = $2
    "#)
    .bind(&payload.post_id)
    .bind(&user_id)
    .execute(&state.db_pool)
    .await;

    match delete_post_query {
        Ok(result) => {
            if result.rows_affected() == 0 {
                return Json(DeleteNewsArticleResponse::fail("No article was deleted".into()));
            } 
        }
        Err(error) => {
            eprintln!("Error: Deleting news article failed while querying for delete: {}, Error: {}", payload.token, error);
            return Json(DeleteNewsArticleResponse::fail("Deleting news internal server error: 1!".into()));
        }
    }

    Json(DeleteNewsArticleResponse::success())
}
