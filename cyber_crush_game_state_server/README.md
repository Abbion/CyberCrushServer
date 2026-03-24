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
