import json
import argparse
import random
from argon2 import PasswordHasher, exceptions
import psycopg2
from psycopg2.extras import Json

PEPPER = ""
MAX_USERNAME_LENGTH = 8
MAX_PASSWORD_LENGTH = 16
MAX_EXTRA_DATA_LENGTH = 8192

DATABASE_NAME = ""
DATABASE_USERNAME = ""
DATABASE_PASSWORD = ""
DATABASE_URL = ""
DATABASE_PORT = 0

def hash_password(password: str, ph: PasswordHasher) -> str:
    return ph.hash(password + PEPPER)

def setup_configuration() -> bool:
    with open("../../server.conf", "r", encoding="utf-8") as file:
        global PEPPER, MAX_UESRNAME_LENGTH, MAX_PASSWORD_LENGTH, MAX_EXTRA_DATA_LENGTH
        global DATABASE_NAME, DATABASE_USERNAME, DATABASE_PASSWORD, DATABASE_URL, DATABASE_PORT

        config = json.load(file)
        PEPPER = config["database_password_pepper"]
        MAX_USERNAME_LENGTH = config["max_username_length"]
        MAX_PASSWORD_LENGTH = config["max_password_length"]
        MAX_EXTRA_DATA_LENGTH = config["max_extra_data_length"]

        DATABASE_NAME = config["database_name"]
        DATABASE_USERNAME = config["database_admin_username"]
        DATABASE_PASSWORD = config["database_admin_password"]
        DATABASE_URL = config["database_url"]
        DATABASE_PORT = config["database_port"]

    if not PEPPER:
        print("No pepper found in the server configuration.")
        return False

    return True

def validate_user(username: str, password: str, extra_data: str) -> bool:
    if len(username) > MAX_USERNAME_LENGTH:
        print(f"Username: { username }, excides { MAX_USERNAME_LENGTH } character limit")
        return False
    if len(password) > MAX_PASSWORD_LENGTH:
        print(f"Password for user { username }, excides { MAX_PASSWORD_LENGTH } character limit")
        return False
    if len(extra_data) > MAX_EXTRA_DATA_LENGTH:
        print(f"Extra data for user { username }, excides { MAX_EXTRA_DATA_LENGTH } character limit")
        return False

    return True

def generate_unique_personal_numbers(count: int) -> list[int]:
    if count > 9000:
        raise ValueError("Error: To many users. Cannot generate unique 4-digit codes.")

    return random.sample(range(1000, 10000), count)

def main():
    parser = argparse.ArgumentParser(description = "Loads user data to the database from .json file")
    parser.add_argument("-f", help = ".json file path")
    args = parser.parse_args()

    if not args.f:
        print("No .json file path provided.")
        return

    file_path = args.f
    password_hasher = PasswordHasher()
    
    if not setup_configuration():
        return
    
    with open(file_path, "r", encoding="utf-8") as file:
        users = json.load(file)

    db_connection = psycopg2.connect(dbname = DATABASE_NAME,
                                         user = DATABASE_USERNAME,
                                         password = DATABASE_PASSWORD,
                                         host = DATABASE_URL,
                                         port = DATABASE_PORT);

    personal_numbers = generate_unique_personal_numbers(len(users))
    username_to_bank_id = {}

    try:
        db_connection.autocommit = False
        db_cursor = db_connection.cursor()

        insert_user_sql = "INSERT INTO users (username, password, user_token, personal_number, extra_data) VALUES (%s, %s, %s, %s, %s) RETURNING id;"
        insert_funds_sql = "INSERT INTO bank_accounts (user_id, funds) VALUES (%s, %s) RETURNING id;"
        insert_bank_transaction_sql = "INSERT INTO bank_transactions (sending_id, receiving_id, message, transaction_amount, time_stamp) VALUES(%s, %s, %s, %s, %s);"


        for (itr, user_data) in enumerate(users):
            username = user_data["username"]
            password = user_data["password"]
            extra_data = user_data.get("extra_data", {})

            if not validate_user(username, password, json.dumps(extra_data, ensure_ascii=False).encode("utf-8")):
                continue

            hashed_password = hash_password(password, password_hasher)
            insert_user_params = (username, hashed_password, None, personal_numbers[itr], Json(extra_data))

            db_cursor.execute(insert_user_sql, insert_user_params)
            user_id = db_cursor.fetchone()[0]

            bank_account = user_data.get("bank_account", {})
            funds = bank_account["funds"]

            insert_funds_params = (user_id, funds)
            db_cursor.execute(insert_funds_sql, insert_funds_params)
            bank_account_id = db_cursor.fetchone()[0]

            username_to_bank_id[username] = bank_account_id

        #Insert bank transactions
        for user_data in users:
            username = user_data["username"]
            sending_id = username_to_bank_id[username]

            for bank_transaction in user_data["bank_transactions"]:
                receiver_username = bank_transaction["receiver"]
                receiving_id = username_to_bank_id.get(receiver_username)

                if receiving_id is None:
                    print(f"No receiver {receiver_username} bank account found!")
                    continue
                
                transaction_amount = bank_transaction["amount"]
                transaction_message = bank_transaction["message"]
                transaction_time_stamp = bank_transaction["time_stamp"]

                insert_bank_transaction_params = (sending_id, receiving_id, transaction_message, transaction_amount, transaction_time_stamp)

                db_cursor.execute(insert_bank_transaction_sql, insert_bank_transaction_params)

        db_connection.commit()
    except Exception:
        if db_connection:
            db_connection.rollback()
        raise
    finally:
        if db_connection:
            db_cursor.close()
            db_connection.close()
        
if __name__ == "__main__":
    main()
