# News Server

The news server handles social media news feed, and posting news articles.

This app uses token-based authentication, meaning each `POST` request must include a valid user token that uniquely identifies the user.

This server connects to a PostgreSQL database and requires proper configuration through `server.conf` in the root repository.

## Endpoints
### **GET** `/hello`

**Description:** Returns "Hello, cyber crush news server!" string.

**Use case:** Simple ping to check if the server is running.

### **GET** `/get_news_feed`

- **Input**: None
- **Output**
    ```json
    {
      "response_status":
      {
        "success": "true/false",
        "message": "string"
      },
      "articles": [
        {
          "author": "string",
          "title": "string",
          "content": "string",
          "timestamp": "datetime"
        }
      ]
    }
- **Description:** 
    - Returns `true` status with a list of 75 latests news articles ordered by `timestamp`
    - Returns `false` status with an empty list of articles if an server error occurs.

### **Post** `/post_news_article`
- **Input**
    ```json
    {
      "token": "string",
      "title": "string",
      "content": "string"
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
    - Returns `true` status if the token is valid and the article is successfully saved in the database.
    - Returns `false` if the token is not valid or an server error occurs.
