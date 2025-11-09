import psycopg2
import json

DATABASE_NAME = ""
DATABASE_USERNAME = ""
DATABASE_PASSWORD = ""
DATABASE_URL = ""
DATABASE_PORT = 0

def setup_configuration():
    global DATABASE_NAME, DATABASE_USERNAME, DATABASE_PASSWORD, DATABASE_URL, DATABASE_PORT
    with open("../../server.conf", "r", encoding="utf-8") as file:
        config = json.load(file)

    DATABASE_NAME = config["database_name"]
    DATABASE_USERNAME = config["database_admin_username"]
    DATABASE_PASSWORD = config["database_admin_password"]
    DATABASE_URL = config["database_url"]
    DATABASE_PORT = config["database_port"]

def clear_database(db_connection):
    try:
        db_cursor = db_connection.cursor()
        db_cursor.execute("""SELECT tablename 
                        FROM pg_tables 
                        WHERE schemaname = 'public';""")

        tables = db_cursor.fetchall()
        if not tables:
            print("No tables found in database.")
        else:
            for (table,) in tables:
                print(f"ropping table: {table}")
                db_cursor.execute(f"DROP TABLE IF EXISTS {table} CASCADE;")
                db_connection.commit()

    except Exception:
        db_connection.rollback()
        raise                    
    finally:
        db_cursor.close()

if __name__ == "__main__":
    setup_configuration()

    db_connection = psycopg2.connect(dbname = DATABASE_NAME,
                                    user = DATABASE_USERNAME,
                                    password = DATABASE_PASSWORD,
                                    host = DATABASE_URL,
                                    port = DATABASE_PORT);

    clear_database(db_connection)
    db_connection.close()
