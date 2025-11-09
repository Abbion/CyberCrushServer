# Authorization Server

The authorization server handles login requests, verifies credentials, and generates tokens.

This app uses token-based authentication, meaning each `POST` request must include a valid user token that uniquely identifies the user.

This server connects to a PostgreSQL database and requires proper configuration through `server.conf` in the root repsitory.

## Endpoints

### **GET** `/hello`
- **Description:** Returns a `"Hello World"` string.
- **Use case:** Simple ping to check if the server is running.

---

### **POST** `/login`
- **Input:**
  ```json
  {
    "username": "string",
    "password": "string"
  }
- **Output**
    ```json
    {
      "response_status": 
      {
        "success": "true/false",
        "message": "string"
      },
      "token": "string (if success)"
    }
- **Description**
    - Returns `true` status and a `token` if login is successful.
    - Returns `false` status and an error message if login fails.
---
### **POST** `/validate_token`
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
      }
    }
- **Description**
    - Returns `true` status if the token is valid.
    - Returns `false` status with an error message if the token is invalid.
