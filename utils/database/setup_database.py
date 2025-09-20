import psycopg2
from psycopg2 import sql

DATABASE_NAME = ""
DATABASE_USERNAME = ""
DATABASE_PASSWORD = ""
DATABASE_URL = ""
DATABASE_PORT = 0

def setup_configuration():
    with open("../../server.conf", "r", encofing="utf-8") as file:
        global DATABASE_NAME, DATABASE_USERNAME, DATABASE_PASSWORD, DATABASE_URL, DATABASE_PORT

        config = json.load(file)
        DATABASE_NAME = config["database_name"]
        DATABASE_USERNAME = config["database_admin_username"]
        DATABASE_PASSWORD = config["database_admin_password"]
        DATABASE_URL = config["database_url"]
        DATABASE_PROT = config["database_port"]

def init_db():
    db_connection = psycopg2.connect(
            dbname = DATABASE_NAME,
            user = DATABASE_USERNAME,
            password = DATABASE_PASSWORD,
            host = DATABASE_URL,
            port = DATABASE_PORT)
    
    try:
        db_cursor = connection.cursor()

        user_table_query = sql.SQL("""
            CREATE TABLE IF NOT EXISTS users (
                id SERIAL PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password TEXT NOT NULL,
                user_token TEXT,
                personal_number CHAR(4) UNIQUE,
                extra_data JSONB
            );
            """)

        db_cursor.execute(user_table_query)
        db_connection.commit()
    except Exception:
        if db_connection:
            db_connection.rollback()
        raise
    finally:
        if db_connection:
            db_cursor.close()
            db_connection.close()
            print("User table created")

if __name__ == "__main__":
    setup_configuration()
    init_db()
