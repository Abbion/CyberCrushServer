# Banking Server
The banking server handles branking transactions, transaction history and user funds.

This app uses token-based authentiaction, meaning each `POST` request must include a valid user token that uniquely identifies the user.

This server connects to a PostgreSQL database and requires proper configuration through `server.conf` in the root repository.

## Endpoints
### GET `/hello`
- **Description:** Returns a ``"Hello, cyber crush bank server!"`` string.
- **Use case:** Simple ping to check if the server is running.

### **POST** `/get_user_funds`
- **Input:**
    ```json
    {
        "token": "string"
    }
- **Output**
    ```json
    {
        "success": true/false,
        "message": "string",
        "funds": "i32"
    }
- **Description**
    - Returns `true` with the user's funds if token is valid.
    - Returns `false` with -1 funds if no data is found or if a server error occurs.

### **POST** `/get_user_transaction_history`
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
        "transactions" : Vec[
            {
                "sernder_username": "string",
                "receiver_username": "string",
                "message": "string",
                "amount": i32,
                "time_step": "naive_date_tinme"
            }
        ]
    }
- **Description**
    - Returns `true` with the user's list of transactions ordered by time_stamp if token is valid.
    - Returns `false` with an empty list of transactions if no data is found or if a server error occurs.

### **Post** `/transfer_funds`
- **Input**
    ```json
    {
        "sender_token": "string",
        "receiver_username": "string",
        "message": "string",
        "amount": i32
    }
- **Output**
    ```json
    {
        "success": true/false,
        "message": "string"
    }
- **Description**
    - Returns `true` if the token is valid and the transaction passes.
    - Returns `false` if transaction can't be completed (token is not valid/no user funds/receiver does not exist/internal server error)
