# Chat Server

The chat server provides access to chat information.
It supports realtime chat, fetching chat information, direct chat, group chat, group chat management.

This app uses token-based authentication, meaning each `POST` request must include a valid user token that uniquely identifies the user.

This server connects to a PostgreSQL database and requires proper configuration through `server.conf` in the root repository.

## Endpoints

### **GET** `/hello`
- **Description:** Returns a `"Hello, cyber crush chat server!"` string.
- **Use case:** Simple ping to check if the server is running.

---
### **POST** `/get_user_chats`
- **Input**
  ```json
  {
    "token": "string"
  }
- **Output**
  ```json
  {
    "response_status": 
    {
      "success": "true/false",
      "message": "string"
    },
    "direct_chats": [
      {
        "chat_id": "i32",
        "chat_partner": "string",
        "last_message": "Optional<string>",
        "last_message_time_stamp": "Optional<NaiveDateTime>"
      }
    ],
    "group_chats": [
      {
        "chat_id": "i32",
        "title": "string",
        "last_message": "Optional<string>",
        "last_message_time_stamp": "Optional<NaiveDateTime>"
      }
    ]
  }
- **Description**
    - Returns `true` status with the user's chats. Each chat prvides the chat id, which can be used for future reqests.
    - Returns `false` status with an error if a server error occurs.
---
###  **POST** `/get_chat_history`
- **Input**
  ```json
  {
    "token": "string",
    "chat_id": "i32",
    "history_time_stamp": "Option<NaiveDateTime>"
  }
- **Output**
  ```json
  {
    "response_status": 
    {
      "success": "true/false",
      "message": "string"
    },
    "messages": [
      {
        "sender": "string",
        "message": "string",
        "time_stamp": "NaiveDateTime"
      }
    ]
  }
- **Description**
    - Returns `true` status with the last 50 messages that were send until the `history_time_stamp` in request. If the `history_time_stamp` is `Null` the latest 50 messages will be returned.
    - Returns `false` status wth an error if a server error occurs.
---
### **POST** `/get_chat_metadata`
- **Input**
  ```json
  {
    "token": "string",
    "chat_id": "i32"
  }
- **Output**
  ```json
  {
    "response_status": 
    {
      "success": "true/false",
      "message": "string"
    },
    "metadata": "Option<ChatMetaData>"
  }

  ChatMetaData
  {
    "Direct/Group": "DirectChatMetaData/GroupChatMetaData"
  }
  
  DirectChatMetaData
  {
    "username_a": "string",
    "username_b": "string"
  }
  
  GroupChatMetaData
  {
    "admin_username": "string",
    "members": ["string"]
  }
- **Description**
    - Returns `true` state with direct chat metadata or group chat medatada.
    - Returns `false` state with an error if chat does not exist or an server error occurs.
---
### **POST** `/update_group_chat_member`
- **Input**
  ```json
  {
    "admin_token": "string",
    "chat_id": "i32",
    "update": {
      "action": "AddMember/DeleteMember",
      "username": "string"
    }
  }
- **Output**
  ```json
  {
    "response_status": 
    {
      "success": "true/false",
      "message": "string"
    }
  }
- **Description**
    - Returns `true` status when update request is finalized.
    - Returns `false` status with an error if chat does not exist or an server error occurs.
---
### **POST** `/create_new_direct_chat`
- **Input**
  ```json
  {
    "token": "string",
    "partner_username": "string",
    "first_message": "string"
  }
- **Output**
  ```json
  {
    "response_status": 
    {
      "success": "true/false",
      "message": "string"
    },
    "chat_id": "Option<i32>"
  }
- **Description**
    - Returns `true` status with an new direct chat id.
    - Returns `false` status with an error if an server error occurs.
---
### **POST** `/create_new_group_chat`
- **INPUT**
  ```json
  {
    "token": "string",
    "title": "string"
  }
- **Output**
  ```json
  {
    "response_status": 
    {
      "success": "true/false",
      "message": "string"
    },
    "chat_id": "Option<i32>"
  }
- **Description**
    - Returns `true` status with an new group chat id.
    - Returns `false` status with an error if an server error occurs.
---
### **GET** `/realtime_chat`
- **Description**

  This endpoint opens a webocket connection with a chat. Users must send `ChatClientMessage::Init{ token: string, chat_id: i32 }` to connect to the chat with a given chat id. On success the server returns `ChatResponse::Info{ text: string }`

  After successful connection the user can send `ChatClientMessage::Msg{ token: String, message: string }` to send messages to the chat or `Exit{ token: string }` to disconnect from the chat.

  User may receive `ChatResponse::ChatMessage{ chat_id: i32, message: string, time_stamp: string}` which contains new chat message, or `ChatResponse::Error{ text: string }` to inform an error occured.

- **Input**
    None
---