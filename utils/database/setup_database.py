import psycopg2
from psycopg2 import sql

database_name = "cc_database"
database_username = "admin"
database_password = "password"
host_ulr = "127.0.0.1"
host_port = 5432

def init_db():
    connection = psycopg2.connect(
            dbname = database_name,
            user = database_username,
            password = database_password,
            host = host_ulr,
            port = host_port
            )

    cursor = connection.cursor()

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

    cursor.execute(user_table_query)
    connection.commit()

    cursor.close()
    connection.close()
    print("User table created")

if __name__ == "__main__":
    init_db()
