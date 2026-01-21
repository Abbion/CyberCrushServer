# Banking Server

The banking server handles banking transactions, transaction history, and user funds.

This app uses token-based authentication, meaning each `POST` request must include a valid user token that uniquely identifies the user.

This server connects to a PostgreSQL database and requires proper configuration through `server.conf` in the root repository.

## Endpoints
### **GET** `/hello`

**Description:** Returns "Hello, cyber crush bank server!" string.

**Use case:** Simple ping to check if the server is running.

### **POST** `/get_user_funds`

- **Input**:
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
      "funds": "i32"
    }
- **Description**
    - Returns `true` status with the user's funds if token is valid.
    - Returns `false` status with -1 funds if no data is found or if a server error occurs.

### **POST** `/get_user_transaction_history`
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
      "transactions": [
        {
          "sernder_username": "string",
          "receiver_username": "string",
          "message": "string",
          "amount": "i32",
          "time_step": "datatime"
        }
      ]
    }
- **Description**
    - Returns `true` status with the user's list of transactions ordered by `time_stamp` if token is valid.
    - Returns `false` status with an empty list of transactions if no data is found or if a server error occurs.

### **Post** `/transfer_funds`
- **Input**
    ```json
    {
      "sender_token": "string",
      "receiver_username": "string",
      "message": "string",
      "amount": "i32"
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
    - Returns `true` status if the token is valid and the transaction passes.
    - Returns `false` status if transaction can't be completed (token is not valid, insufficient user funds, receiver does not exist, or an internal server error occurs)
