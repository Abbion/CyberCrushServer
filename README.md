# Cybercrush backend
##### version: indev 0.1v
This project was created to implement software-based user interactions for the **Cybercrush LARP** event. This repository only contains the backend of the project, the mobile app is available at ....

The servers are written in Rust using Tokio and Axum. On the database side, PostgreSQL was chosen. Python scripts run utility tools. For more information about each server or tool, navigate to the appropriate folder.

For a top down view of the backend design, check out the UML files.

### Features
Each feature is supported by its own dedicated server:
 - User authentication
 - Banking
 - User data provider
 - News feed
 - Direct messaging and group messaging

## Build and run
#### Prerequisites
Make sure you have all of these tools installed:
 - Rust
 - Python3
 - Postgresql

#### Configuration file
In the repository root folder, you will find a file called `server.conf`. This file holds important server/database configuration variables like ports or password pepper.

#### Database
Before running the servers, it is recommended to create the database tables and fill them with data. To do so, go to the `utils/database` and run the `setup_database.py` script (make sure you install all the required libraries. The `utls/database/README.md` covers those). After the tables are created, run the `user_loader.py script` with the flag `-f` and `example_users.json` as its argument. Go back to the repository root folder. There, find the `server.conf` file, in it, you will find the *database name, URL, admin username, and admin password*. Use those parameters to log in to the database and check the tables and their contents.

#### Servers
Servers are independent of each other, so you can run them in any sequence. Just enter the server folder and call `cargo run`.

## Contributing
To contribute, create a new branch using the snake_case naming convention and create a pull request. You can also fork the project and create a pull request from that.

### Coding conventions
To improve on the code review process please adhere to those rules:
1. Use snake_case naming convention for variables, functions, directories, and files.
2. For structures or classes, use the CamelCase naming convention.
3. Don't use comments if not necessary, code should describe itself.
4. Prioritize clarity over "cleverness".
5. When placing braces, use the **Kernighan & Ritchie style**.
6. **Never unwrap!** Handle the error and log it to the console using the `eprintln!` macro. Start with *Error:*, continue with a unique description, and print the error message provided by the `Err()`.

## Addtional information and future featurs
All the README and UML files will be frquently updated to reflect the present project state. 

#### Future features
 - A setup script will be provided that handles the whole backend setup.
 - Docker setup.
