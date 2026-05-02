# Hacking Server

The hacking server provides access to hacking actions.
It supports checking hacker status, getting victim token, logging hack actions.

This app uses token-based authentication, meaning each `POST` request must include a valid user personal_number that uniquely identifies the user.

This server connects to a PostgreSQL database and requires proper configuration through `server.conf` in the root repository.

## Endpoints

### **GET** `/hello`
- **Description:** Returns a `"Hello, cyber crush hacking server!"` string.
- **Use case:** Simple ping to check if the server is running.

---
### **POST** `/get_hacker_info`
- **Input**
  ```json
  {
    "personal_number": "string"
  }
- **Output**
  ```json
  {
    "response_status": 
    {
      "success": "true/false",
      "status_message": "string"
    },
    "hacker_info":
    {
      "username": "string",
      "can_hack": "true/false"
    }
  }
- **Description**
    The personal number is a unique 4 digit number in the user id panel dashboard.
    - Returns `true` status with the hacker basic information.
    - Returns `false` status with a message if a server error occurs.

---
### **GET** `/get_hackable_users`
- **Output**
  ```json
  {
    "response_status": 
    {
      "success": "true/false",
      "status_message": "string"
    },
    "users": 
    [
      {
        "username": "string",
        "cyber_defence_level": i32,
        "personal_number": i32
      },
      ...
    ]
  }
- **Description**
    The personal number is a unique 4 digit number in the user id panel dashboard.
    - Returns `true` status with a list of hackable users.
    - Returns `false` status with a message if a server error occurs.

---
### **POST** `/get_available_hack_types`
- **Input**
  ```json
  {
    "hacker_personal_number": "string",
    "victim_personal_number": "string"
  }
- **Output**
  ```json
  {
    "response_status": 
    {
      "success": "true/false",
      "status_message": "string"
    },
    "available_hacks": 
    [
      "PersonalData",
      "ChatAccess",
      {
        "ChatData": 0
      },
      {
        "BankTransaction": 0
      }
    ]
  }
- **Description**
    The personal number is a unique 4 digit number in the user id panel dashboard. The **ChataData** and **BankTransaction** actions always return 0 as their parameter. Those parameters need to be filled while logging the hack state result.
    - Returns `true` status with a list of hacking actions.
    - Returns `false` status with a message if a server error occurs.

---
### **POST** `/get_available_hack_types`
- **Input**
  ```json
  {
    "victim_personal_number": "string"
  }
- **Output**
  ```json
  {
    "response_status": 
    {
      "success": "true/false",
      "status_message": "string"
    },
    "token": "string"
  }
- **Description**
    The personal number is a unique 4 digit number in the user id panel dashboard. **NEVER!** store the token in the hacking client termianl. Request it before using and then discard.
    - Returns `true` status with a victim token.
    - Returns `false` status with a message if a server error occurs.

---
### **POST** `/get_available_hack_types`
- **Input**
  ```json
  {
    "hacker_personal_number": "string",
    "victim_personal_number": "string",
    "hack_type": HackType,
    "hack_successful": "true/false"
  }
  HackType
  [
    "PersonalData"/
    "ChatAccess"/
    {
      "ChatData": 0
    }/
    {
      "BankTransaction": 0
    }
  ]
- **Output**
  ```json
  {
    "success": "true/false",
    "status_message": "string"
  }
- **Description**
    The personal number is a unique 4 digit number in the user id panel dashboard. The HackType is a Rust enum and can be one of the listed values.
    - Returns `true` if the state was logged.
    - Returns `false` status with a message if a server error occurs.