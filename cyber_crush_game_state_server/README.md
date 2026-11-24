# Game state Server

The game state server informs about the state of the game by provideing a game state structure with all necessary values.

## Endpoints

### **GET** `/hello`
- **Description:** Returns a `"Hello World"` string.
- **Use case:** Simple ping to check if the server is running.

---
### **GET** `/game_state`
- **Output**
    ```json
    {
      "is_game_online": "true/false",
      "info_panel_text": "string",
    }

---
### **GET** `/admin`
- **Description:** A single web page that does everything the terminal interface does: switch the game online/offline and set the offline message (custom or default). It refreshes every 3 seconds.
- **Use case:** Managing the game state when the server runs in Docker, where there is no terminal.

---
### **POST** `/admin/game_state`
- **Input** (both fields are optional, empty text is ignored)
    ```json
    {
      "is_game_online": "true/false",
      "info_panel_text": "string"
    }
    ```
- **Output:** The new game state, in the same format as `GET /game_state`.

## Admin authentication
`/admin` and `/admin/game_state` use HTTP Basic auth. Set `admin_username` and `admin_password` in the configuration file of each environment (`server.conf` for local runs, `server.docker.conf` for Docker). **Change the default password** before any real event. If either value is empty, both admin endpoints answer `503` and stay disabled. `GET /game_state` stays public.

## Terminal interface
When started in a real terminal (`cargo run`), the server also shows the ratatui menu. Without a terminal (for example in Docker), it skips the menu and only the web page is available.
