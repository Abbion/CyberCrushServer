# Database Utility Tools

This folder contains Python scripts for managing the PostgreSQL database and loading data.
Before running the scripts, make sure the following Python 3 libraries are installed:

- `psycopg2`
- `argon2`

---

### **setup_database.py**
- **Description:** Creates the necessary tables and indexes for the PostgreSQL database.  
- **Usage:** Run once when initializing the database.

---

### **user_loader.py**
- **Description:** Loads JSON-formatted user data into the PostgreSQL database.
- **Parameters:**
  - `-f <path>` → Path to the JSON data file.

**Example:**
```bash
python3 user_loader.py -f ./example_users.json
