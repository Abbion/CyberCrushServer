import psycopg2
from psycopg2 import sql
import json

DATABASE_NAME = ""
DATABASE_USERNAME = ""
DATABASE_PASSWORD = ""
DATABASE_URL = ""
DATABASE_PORT = 0

def setup_configuration():
    with open("../../server.conf", "r", encoding="utf-8") as file:
        global DATABASE_NAME, DATABASE_USERNAME, DATABASE_PASSWORD, DATABASE_URL, DATABASE_PORT

        config = json.load(file)
        DATABASE_NAME = config["database_name"]
        DATABASE_USERNAME = config["database_admin_username"]
        DATABASE_PASSWORD = config["database_admin_password"]
        DATABASE_URL = config["database_url"]
        DATABASE_PORT = config["database_port"]

def init_db():
    db_connection = psycopg2.connect(
            dbname = DATABASE_NAME,
            user = DATABASE_USERNAME,
            password = DATABASE_PASSWORD,
            host = DATABASE_URL,
            port = DATABASE_PORT)
    
    try:
        db_cursor = db_connection.cursor()

        user_table_query = sql.SQL("""
            CREATE TABLE IF NOT EXISTS users (
                id SERIAL PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                password TEXT NOT NULL,
                user_token TEXT,
                personal_number INT NOT NULL UNIQUE,
                extra_data JSONB
            );
            """)

        db_cursor.execute(user_table_query)

        bank_account_table_query = sql.SQL("""
            CREATE TABLE IF NOT EXISTS bank_accounts (
                id SERIAL PRIMARY KEY,
                user_id INTEGER NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
                funds INTEGER NOT NULL
            );
            """)

        db_cursor.execute(bank_account_table_query)

        bank_transactions_table_query = sql.SQL("""
            CREATE TABLE IF NOT EXISTS bank_transactions (
                id SERIAL PRIMARY KEY,
                sender_id INTEGER NOT NULL REFERENCES bank_accounts(id),
                receiver_id INTEGER NOT NULL REFERENCES bank_accounts(id),
                message TEXT NOT NULL,
                amount INTEGER NOT NULL,
                time_stamp TIMESTAMP NOT NULL DEFAULT NOW()

            );
        """)

        db_cursor.execute(bank_transactions_table_query)
        
        bank_account_id_to_user_id_index = sql.SQL("""
            CREATE INDEX index_bank_accounts_user_id ON bank_accounts(user_id);
        """)

        db_cursor.execute(bank_account_id_to_user_id_index)

        # TODO ADD INDEXES to help with the queries
        # Join on to get the usernames SELECT t.id, u1.username AS sender, u2.username AS receiver, 
        # t.transaction_amount, t.time_stamp
        # FROM bank_transactions t
        # JOIN users u1 ON t.sending_id = u1.id
        # JOIN users u2 ON t.receiving_id = u2.id;

        db_connection.commit()
    except Exception:
        if db_connection:
            db_connection.rollback()
        raise
    finally:
        if db_connection:
            db_cursor.close()
            db_connection.close()
            print("Script finished. State success!")

if __name__ == "__main__":
    setup_configuration()
    init_db()
