use shared_server_lib::{common, common::ResponseStatus};

use axum::{
    extract::{Json, State},
    response::IntoResponse,
};

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use chrono;

use crate::common_chat::{ ServerState, ChatType };
use crate::common_chat;

#[derive(Debug, Deserialize)]
pub struct GetUserChatsRequest {
    token: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DirectChat {
    chat_id: i32,
    chat_partner: String,
    last_message: Option<String>,
    last_message_time_stamp: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct GroupChat {
    chat_id: i32,
    title: String,
    last_message: Option<String>,
    last_message_time_stamp: Option<chrono::NaiveDateTime>
}

#[derive(Debug, Serialize)]
pub struct GetUserChatsResponse {
    response_status: ResponseStatus,
    direct_chats: Option<Vec<DirectChat>>,
    group_chats: Option<Vec<GroupChat>>
}

#[derive(Debug, Deserialize)]
pub struct GetChatHistoryRequest {
    token: String,
    chat_id: i32,
    history_time_stamp: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ChatMessage {
    sender: String,
    message: String,
    time_stamp: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct GetChatHistoryResponse {
    response_status: ResponseStatus,
    messages: Vec<ChatMessage>,
}

impl GetChatHistoryResponse {
    fn fail(reason: &str) -> GetChatHistoryResponse {
        GetChatHistoryResponse{
            response_status: ResponseStatus::fail(reason.into()),
            messages: vec![] }
    }

    fn success(messages: Vec<ChatMessage>) -> GetChatHistoryResponse {
        GetChatHistoryResponse{
            response_status: ResponseStatus::success(),
            messages }
    }
}

#[derive(Debug, Deserialize)]
pub struct GetChatMetaDataRequest {
    token: String,
    chat_id: i32,
}

#[derive(Debug, Serialize)]
pub struct DirectChatMetaData {
    username_a: String,
    username_b: String,
}

#[derive(Debug, Serialize)]
pub struct GroupChatMetaData {
    admin_username: String,
    members: Vec<String>,
}

#[derive(Debug, Serialize)]
enum ChatMetaData {
    Direct(DirectChatMetaData),
    Group(GroupChatMetaData),
}

#[derive(Debug, Serialize)]
pub struct GetChatMetaDataResponse {
    response_status: ResponseStatus,
    chat_meta_data: Option<ChatMetaData>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", content = "username")]
enum GroupMemberUpdate {
    AddMember(String),
    DeleteMember(String),
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupChatMemberRequest {
    admin_token: String,
    chat_id: i32,
    update: GroupMemberUpdate,
}

use ResponseStatus as UpdateGroupChatMemberResponse;

#[derive(Debug, Deserialize)]
pub struct CreateNewDirectChatRequest {
    token: String,
    partner_username: String,
    first_message: String,
}

#[derive(Debug, Serialize)]
pub struct CreateNewDirectChatResponse {
    response_status: ResponseStatus,
    chat_id: Option<i32>,
}

impl CreateNewDirectChatResponse {
    fn fail(reason: &str) -> CreateNewDirectChatResponse {
        CreateNewDirectChatResponse{
            response_status: ResponseStatus::fail(reason.into()),
            chat_id: None }
    }

    fn success(chat_id: i32) -> CreateNewDirectChatResponse {
        CreateNewDirectChatResponse{
            response_status: ResponseStatus::success(),
            chat_id: Some(chat_id) }
    }

    fn chat_exists(chat_id: i32) -> CreateNewDirectChatResponse {
        CreateNewDirectChatResponse{
            response_status: ResponseStatus::fail("Direct chat already exsits!".into()),
            chat_id: Some(chat_id) }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateNewGroupChatRequest {
    token: String,
    title: String
}

#[derive(Debug, Serialize)]
pub struct CreateNewGroupChatResponse {
    response_status: ResponseStatus,
    chat_id: Option<i32>,
}

pub async fn hello() -> &'static str {
    "Hello, cyber crush chat server!"
}

pub async fn get_user_chats(State(state): State<Arc<ServerState>>, Json(payload): Json<GetUserChatsRequest>) -> impl IntoResponse {
    let validated = common::validate_token(&state.db_pool, &payload.token).await;
    
    if validated.response_status.success == false {
        return Json(GetUserChatsResponse{ response_status: ResponseStatus::fail("User not validated".into()), direct_chats: None, group_chats: None });
    }

    let user_id = validated.id.unwrap();
    
    let direct_chats_query = sqlx::query_as::<_, DirectChat>(
    r#"
        SELECT dc.chat_id, u.username AS chat_partner, dc.last_message, dc.last_time_stamp AS last_message_time_stamp
        FROM direct_chats dc
        JOIN user_chats uc1 ON uc1.chat_id = dc.chat_id
        JOIN user_chats uc2 ON uc2.chat_id = dc.chat_id
        JOIN users u ON u.id = uc2.user_id
        WHERE uc1.user_id = $1 AND uc2.user_id != $1
    "#)
    .bind(user_id)
    .fetch_all(&state.db_pool)
    .await;

    let direct_chats = match direct_chats_query {
        Ok(chats) => chats,
        Err(error) => {
            eprintln!("Error: Getting user chats failed while querying direct chats for user id: {}, error: {}", user_id, error);
            return Json(GetUserChatsResponse{ response_status: ResponseStatus::fail("Internal server error: 1".into()), direct_chats: None, group_chats: None });
        }
    };

    let group_chats_query = sqlx::query_as::<_, GroupChat>(
    r#"
        SELECT gc.chat_id, gc.title AS title, gc.last_message, gc.last_time_stamp AS last_message_time_stamp
        FROM group_chats gc
        JOIN user_chats uc ON uc.chat_id = gc.chat_id
        WHERE uc.user_id = $1
    "#)
    .bind(user_id)
    .fetch_all(&state.db_pool)
    .await;

    let group_chats = match group_chats_query {
        Ok(chats) => chats,
        Err(error) => {
            eprintln!("Error: Getting user chats failed while querying group chats for user id: {}, error: {}", user_id, error);
            return Json(GetUserChatsResponse{ response_status: ResponseStatus::fail("Internal server error: 2".into()), direct_chats: None, group_chats: None });
        }
    };

    Json(GetUserChatsResponse{ response_status: ResponseStatus::success(), direct_chats: Some(direct_chats), group_chats: Some(group_chats) })
}

pub async fn get_chat_history(State(state): State<Arc<ServerState>>, Json(payload): Json<GetChatHistoryRequest>) -> impl IntoResponse {
    let validated = common::validate_token(&state.db_pool, &payload.token).await;
    
    if validated.response_status.success == false {
        return Json(GetChatHistoryResponse::fail("Token validation failed".into()));
    }

    let membership_query = sqlx::query_scalar::<_, i32>(
    r#"
        SELECT 1 FROM user_chats WHERE chat_id = $1 AND user_id = $2      
    "#)
    .bind(payload.chat_id)
    .bind(validated.id)
    .fetch_optional(&state.db_pool)
    .await;

    let membership_check = match membership_query {
        Ok(member) => member,
        Err(error) => {
            eprintln!("Error: Checking user chat membership failed for user id: {:?} in chat id {}, Error: {}", validated.id, payload.chat_id, error);
            return Json(GetChatHistoryResponse::fail("Internal server error: 1".into()));
        }
    };

    if membership_check.is_none() {
        return Json(GetChatHistoryResponse::fail("Not a chat member".into()));
    }

    let message_query = match payload.history_time_stamp {
        Some(time_stamp) => {
            sqlx::query_as::<_, ChatMessage>(
            r#"
                SELECT u.username AS sender, m.content AS message, m.time_stamp
                FROM chat_messages m
                JOIN users u ON u.id = m.sender_id
                WHERE m.chat_id = $1 AND m.time_stamp < $2
                ORDER BY m.time_stamp DESC
                LIMIT 50
            "#)
            .bind(payload.chat_id)
            .bind(time_stamp)
            .fetch_all(&state.db_pool)
        },
        None => {
            sqlx::query_as::<_, ChatMessage>(
            r#"
                SELECT u.username AS sender, m.content AS message, m.time_stamp
                FROM chat_messages m
                JOIN users u ON u.id = m.sender_id
                WHERE m.chat_id = $1
                ORDER BY m.time_stamp DESC
                LIMIT 50
            "#)
            .bind(payload.chat_id)
            .fetch_all(&state.db_pool)
        }
    };

    let response = match message_query.await {
        Ok(mut messages) => {
            messages.reverse();
            GetChatHistoryResponse::success(messages)
        },
        Err(error) => {
            eprintln!("Error: Getting user chat history failed for user id: {:?} and chat id: {}, error: {}", validated.id, payload.chat_id, error);
            GetChatHistoryResponse::fail("Internal server error: 2".into())
        }
    };
    
    Json(response)
}

pub async fn get_chat_metadata(State(state): State<Arc<ServerState>>, Json(payload): Json<GetChatMetaDataRequest>) -> impl IntoResponse {
    let validated = common::validate_token(&state.db_pool, &payload.token).await;
    
    if validated.response_status.success == false {
        return Json(GetChatMetaDataResponse{ response_status: ResponseStatus::fail("Token validation failed".into()), chat_meta_data:  None });
    }

    let membership_query = sqlx::query_scalar::<_, i32>(
    r#"
        SELECT 1 FROM user_chats WHERE chat_id = $1 AND user_id = $2;        
    "#)
    .bind(payload.chat_id)
    .bind(validated.id)
    .fetch_optional(&state.db_pool)
    .await;

    let membership_check = match membership_query {
        Ok(member) => member,
        Err(error) => {
            eprintln!("Error: Checking user chat membership failed for user id: {:?} and chat id: {}, error: {}", validated.id, payload.chat_id, error);
            return Json(GetChatMetaDataResponse{ response_status: ResponseStatus::fail("Internal server error: 1".into()), chat_meta_data:  None });
        }
    };

    if membership_check.is_none() {
        return Json(GetChatMetaDataResponse{ response_status: ResponseStatus::fail("Not a chat member".into()), chat_meta_data:  None });
    }
    
    let chat_type = match common_chat::get_chat_type(&state.db_pool, payload.chat_id).await {
        Ok(chat_type) => chat_type,
        Err(error) => {
            return Json(GetChatMetaDataResponse{ response_status: ResponseStatus::fail(error), chat_meta_data:  None });
        }
    };

    let response = match chat_type {
        ChatType::Direct => {
            let users_query = sqlx::query_scalar::<_, String>(
            r#"
                SELECT u.username FROM users u JOIN user_chats uc ON uc.user_id = u.id WHERE uc.chat_id = $1
            "#)
            .bind(payload.chat_id)
            .fetch_all(&state.db_pool)
            .await;
            
            let users = match users_query {
                Ok(users) => users,
                Err(error) => {
                    eprintln!("Error: Getting direct chat members usernames failed for chat id: {}, error: {}", payload.chat_id, error);
                    return Json(GetChatMetaDataResponse{ response_status: ResponseStatus::fail("Internal server error: 3".into()), chat_meta_data:  None });
                }
            };

            if users.len() != 2 {
                return Json(GetChatMetaDataResponse{ response_status: ResponseStatus::fail("Invalid user amount".into()), chat_meta_data: None });
            }

            let direct_chat_meta_data = ChatMetaData::Direct(DirectChatMetaData{ username_a: users[0].clone(), username_b: users[1].clone() });
            GetChatMetaDataResponse{ response_status: ResponseStatus::success(), chat_meta_data: Some(direct_chat_meta_data) }
        },
        ChatType::Group => {
            let admin_query = sqlx::query_scalar::<_, String>(
            r#"
                SELECT u.username AS username
                FROM group_chats gc
                JOIN users u ON u.id = gc.admin_id
                WHERE gc.chat_id = $1
            "#
            )
            .bind(payload.chat_id)
            .fetch_optional(&state.db_pool)
            .await;

            let admin_username = match admin_query {
                Ok(Some(username)) => username,
                Ok(None) => {
                    return Json(GetChatMetaDataResponse{ response_status: ResponseStatus::fail("Admin not found".into()), chat_meta_data:  None });
                },
                Err(error) => {
                    eprintln!("Error: Getting group chat admin username failed for chat id: {}, error: {}", payload.chat_id, error);
                    return Json(GetChatMetaDataResponse{ response_status: ResponseStatus::fail("Internal server error: 4".into()), chat_meta_data:  None });
                }
            };

            let members_query = sqlx::query_scalar::<_, String>(
            r#"
                SELECT u.username
                FROM user_chats uc
                JOIN users u ON u.id = uc.user_id
                WHERE uc.chat_id = $1
            "#)
            .bind(payload.chat_id)
            .fetch_all(&state.db_pool)
            .await;
            
            match members_query {
                Ok(members) => {
                    let group_chat_meta_data = ChatMetaData::Group(GroupChatMetaData{ admin_username, members });
                    GetChatMetaDataResponse{ response_status: ResponseStatus::success(), chat_meta_data: Some(group_chat_meta_data) }
                },
                Err(error) => {
                    eprintln!("Error: Getting group chat members failed for chat id: {}, error: {}", payload.chat_id, error);
                    return Json(GetChatMetaDataResponse{ response_status: ResponseStatus::fail("Internal server error: 5".into()), chat_meta_data:  None });
                }
            }
        }
    };

    Json(response)
}

pub async fn update_group_chat_member(State(state): State<Arc<ServerState>>, Json(payload): Json<UpdateGroupChatMemberRequest>) -> impl IntoResponse {
    let validated = common::validate_token(&state.db_pool, &payload.admin_token).await;
    
    if validated.response_status.success == false {
        return Json(UpdateGroupChatMemberResponse::fail("Token validation failed".into()));
    }

    let validate_admin_query = sqlx::query_scalar::<_, i64>(
    r#"
        SELECT COUNT(*) FROM group_chats WHERE chat_id = $1 AND admin_id = $2
    "#)
    .bind(payload.chat_id)
    .bind(validated.id)
    .fetch_one(&state.db_pool)
    .await;

    match validate_admin_query {
        Ok(0) => {
            return Json(UpdateGroupChatMemberResponse::fail("Not a group admin".into()))
        },
        Err(error) => {
            eprintln!("Error: Group member update failed for user id: {:?} and chat id: {}, error: {}", validated.id, payload.chat_id, error);
            return Json(UpdateGroupChatMemberResponse::fail("Internal server error: 1".into()))
        },
        _ => {}
    }

    match payload.update {
        GroupMemberUpdate::AddMember(username) => {
            let add_member_query = sqlx::query(
            r#"
                INSERT INTO user_chats (chat_id, user_id) 
                VALUES (
                    $1,
                    (SELECT id FROM users WHERE username = $2)
                )
                ON CONFLICT DO NOTHING
            "#)
            .bind(payload.chat_id)
            .bind(&username)
            .execute(&state.db_pool)
            .await;
            
            match add_member_query {
                Ok(_) => Json(UpdateGroupChatMemberResponse::success()),
                Err(error) => {
                    eprintln!("Error: Group member update failed to add a new member for user: {} and chat id: {}, error: {}", username, payload.chat_id, error);
                    Json(UpdateGroupChatMemberResponse::fail("Internal server error: 2".into()))
                }
            }
        },
        GroupMemberUpdate::DeleteMember(username) => {
            let delete_member_query = sqlx::query(
            r#"
                DELETE FROM user_chats 
                WHERE chat_id = $1 
                    AND user_id = (SELECT id FROM users WHERE username = $2)
                    AND user_id NOT IN (SELECT admin_id FROM group_chats WHERE chat_id = $1)
            "#)
            .bind(payload.chat_id)
            .bind(&username)
            .execute(&state.db_pool)
            .await;

            match delete_member_query {
                Ok(_) => Json(UpdateGroupChatMemberResponse::success()),
                Err(error) => {
                    eprintln!("Error: Group member update failed to delete member for user: {} and chat_id: {}, error: {}", username, payload.chat_id, error);
                    Json(UpdateGroupChatMemberResponse::fail("Internal server error: 3".into()))
                }
            }
        }
    }
}

pub async fn create_new_direct_chat(State(state): State<Arc<ServerState>>, Json(payload): Json<CreateNewDirectChatRequest>) -> impl IntoResponse {
    let validated = common::validate_token(&state.db_pool, &payload.token).await;
    
    if validated.response_status.success == false {
        return Json(CreateNewDirectChatResponse::fail("Token validation failed".into()));
    }

    let sender_id = validated.id.unwrap();

    let partner_id = common::get_user_id_by_username(&state.db_pool, &payload.partner_username).await;

    if partner_id.response_status.success == false {
        return Json(CreateNewDirectChatResponse::fail("Partner does not exist".into()));
    }

    let partner_id = partner_id.id.unwrap();

    let mut transaction = match state.db_pool.begin().await {
        Ok(tx) => tx,
        Err(error) => {
            eprintln!("Error: Creating new direct chat failed while creating transaction for user id: {}, error: {}", sender_id, error);
            return Json(CreateNewDirectChatResponse::fail("Internal server error: 1".into()));
        }
    };

    let chat_instance_check_query = sqlx::query_scalar::<_, i32>(
    r#"
        SELECT dc.chat_id
        FROM direct_chats dc
        JOIN user_chats uc1 ON uc1.chat_id = dc.chat_id
        JOIN user_chats uc2 ON uc2.chat_id = dc.chat_id
        WHERE uc1.user_id = $1 AND uc2.user_id = $2
    "#)
    .bind(sender_id)
    .bind(partner_id)
    .fetch_optional(&mut *transaction)
    .await;

    if let Ok(Some(chat_id)) = chat_instance_check_query {
        let _ = transaction.rollback().await;
        return Json(CreateNewDirectChatResponse::chat_exists(chat_id));
    }
    
    let chat_id_query = sqlx::query_scalar::<_, i32>(
    r#"
        INSERT INTO chats DEFAULT VALUES RETURNING id
    "#)
    .fetch_one(&mut *transaction)
    .await;

    let chat_id = match chat_id_query {
        Ok(id) => id,
        Err(error) => {
            eprintln!("Error: Creating new direct chat failed while creating new chat for user id: {}, error: {}", sender_id, error);
            let _ = transaction.rollback().await;
            return Json(CreateNewDirectChatResponse::fail("Internal server error: 2"));
        }
    };

    let add_chat_for_users_query = sqlx::query(
    r#"
        INSERT INTO user_chats (chat_id, user_id) VALUES ($1, $2), ($1, $3)
    "#)
    .bind(chat_id)
    .bind(sender_id)
    .bind(partner_id)
    .execute(&mut *transaction)
    .await;

    if let Err(error) = add_chat_for_users_query {
        eprintln!("Error: Creating new direct chat failed while attaching to users for sender id: {} and partner id: {}, error: {}", sender_id, partner_id, error);
        let _ = transaction.rollback().await;
        return Json(CreateNewDirectChatResponse::fail("Internal server error: 3"));
    }

    let time_stamp = chrono::Utc::now().naive_utc();
    let add_first_message_query = sqlx::query(
    r#"
        INSERT INTO chat_messages (chat_id, sender_id, content, time_stamp)
        VALUES ($1, $2, $3, $4)
    "#)
    .bind(chat_id)
    .bind(sender_id)
    .bind(payload.first_message.clone())
    .bind(time_stamp)
    .execute(&mut *transaction)
    .await;

    if let Err(error) = add_first_message_query {
        eprintln!("Error: Creating new direct chat failed while adding first message to a new direct chat for sender id: {} and partner id: {}, error: {}", sender_id, partner_id, error);
        let _ = transaction.rollback().await;
        return Json(CreateNewDirectChatResponse::fail("Internal server error: 4"));
    }
    
    let create_direct_chat_query = sqlx::query(
    r#"
        INSERT INTO direct_chats (chat_id, last_message, last_time_stamp)
        VALUES ($1, $2, $3)
    "#)
    .bind(chat_id)
    .bind(payload.first_message)
    .bind(time_stamp)
    .execute(&mut *transaction)
    .await;

    if let Err(error) = create_direct_chat_query {
        eprintln!("Error: Creating new direct chat failed for sender id: {} and partner id: {}, error: {}", sender_id, partner_id, error);
        let _ = transaction.rollback().await;
        return Json(CreateNewDirectChatResponse::fail("Internal server error: 5"));
    }

    if let Err(error) = transaction.commit().await {
        eprintln!("Error: Creating new direct chat failed while commiting transaction for sender id: {} and partner id: {}, error: {}", sender_id, partner_id, error);
        return Json(CreateNewDirectChatResponse::fail("Internal server error: 5"));
    }

    Json(CreateNewDirectChatResponse::success(chat_id))
}

pub async fn create_new_group_chat(State(state): State<Arc<ServerState>>, Json(payload): Json<CreateNewGroupChatRequest>) -> impl IntoResponse {
    let validated = common::validate_token(&state.db_pool, &payload.token).await;
    
    if validated.response_status.success == false {
        return Json(CreateNewGroupChatResponse{ response_status: ResponseStatus::fail("Token validation failed".into()), chat_id: None });
    }

    let admin_id = validated.id.unwrap();

    let mut transaction = match state.db_pool.begin().await {
        Ok(tx) => tx,
        Err(error) => {
            eprintln!("Error: Creating new direct chat failed while creating transaction for user id: {}, error: {}", admin_id, error);
            return Json(CreateNewGroupChatResponse{ response_status: ResponseStatus::fail("Internal server error: 1".into()), chat_id: None });
        }
    };

    let chat_id_query = sqlx::query_scalar::<_, i32>(
    r#"
        INSERT INTO chats DEFAULT VALUES RETURNING id
    "#)
    .fetch_one(&mut *transaction)
    .await;

    let chat_id = match chat_id_query {
        Ok(id) => id,
        Err(error) => {
            eprintln!("Error: Creating new group chat failed while creating new chat for user id: {}, error: {}", admin_id, error);
            let _ = transaction.rollback().await;
            return Json(CreateNewGroupChatResponse{ response_status: ResponseStatus::fail("Internal server error: 2".into()), chat_id: None });
        }
    };

    let assign_admin_to_chat_query = sqlx::query(
    r#"
        INSERT INTO user_chats (chat_id, user_id) VALUES ($1, $2)
    "#)
    .bind(chat_id)
    .bind(admin_id)
    .execute(&mut *transaction)
    .await;

    if let Err(error) = assign_admin_to_chat_query {
        eprintln!("Error: Creating new group chat failed while attaching admin for user id: {}, error: {}", admin_id, error);
        let _ = transaction.rollback().await;
        return Json(CreateNewGroupChatResponse{ response_status: ResponseStatus::fail("Internal server error: 3".into()), chat_id: None });
    }

    let create_group_chat_query = sqlx::query(
    r#"
        INSERT INTO group_chats (chat_id, admin_id, title, last_message, last_time_stamp)
        VALUES ($1, $2, $3, NULL, NULL)
    "#)
    .bind(chat_id)
    .bind(admin_id)
    .bind(payload.title)
    .execute(&mut *transaction)
    .await;

    if let Err(error) = create_group_chat_query {
        eprintln!("Error: Creating new group chat failed for user id: {}, error: {}", admin_id, error);
        let _ = transaction.rollback().await;
        return Json(CreateNewGroupChatResponse{ response_status: ResponseStatus::fail("Internal server error: 4".into()), chat_id: None })
    }

    if let Err(error) = transaction.commit().await {
        eprintln!("Error: Creating new group chat failed while commiting transaction for user id: {}, error: {}", admin_id, error);
        return Json(CreateNewGroupChatResponse{ response_status: ResponseStatus::fail("Internal server error: 5".into()), chat_id: None });
    }

    Json(CreateNewGroupChatResponse{ response_status: ResponseStatus::success(), chat_id: Some(chat_id) })
}

