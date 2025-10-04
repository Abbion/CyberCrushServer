# Data Server

The data server provides access to user-related information.
It supports retrieving all usernames and fetching detailed user information.

This app uses token-based authentication, meaning each `POST` request must include a valid user token that uniquely identifies the user.

This server connects to a PostgreSQL database and requires proper configuration through `server.conf` in the root repository.

## Endpoints

### **GET** `/hello`
- **Description:** Returns a `"Hello, cyber crush data server!"` string.
- **Use case:** Simple ping to check if the server is running.

---

### **GET** `/get_all_usernames`
- **Input:** None
- **Output:**
  ```json
  {
    "success": true/false,
    "message": "string",
    "usernames": ["string", "string", "..."]
  }
- **Description**
    - Returns `true` with a list of usernames if successful.
    - Returns `false` with an error message if the database query fails.
---
### **POST** `/get_user_data`
- **Input**
  ```json
  {
    "token": "string"
  }
- **Output**
  ```json
  {
    "success": true/false,
    "message": "string",
    "username": "string",
    "personal_number": "string",
    "extra_data": "json-string"
  }
 - **Description**
    - Returns `true` with the user's data if token is valid.
    - Returns `false` with an error message if no data is found or if a server error occurs.
