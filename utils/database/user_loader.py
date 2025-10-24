import json
import argparse
import random
from argon2 import PasswordHasher, exceptions
import psycopg2
from psycopg2 import sql
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

def load_user_data(user_data_path, db_connection):
    with open(user_data_path, "r", encoding="utf-8") as file:
        users = json.load(file)

    password_hasher = PasswordHasher()
    personal_numbers = generate_unique_personal_numbers(len(users))

    username_to_bank_id = {}
    username_to_id = {}

    try:
        db_cursor = db_connection.cursor()

        insert_user_sql = """INSERT INTO users (username, password, user_token, personal_number, extra_data)
                            VALUES (%s, %s, %s, %s, %s) RETURNING id;"""
        insert_funds_sql = """INSERT INTO bank_accounts (user_id, funds)
                                VALUES (%s, %s) RETURNING id;"""
        insert_bank_transaction_sql = """INSERT INTO bank_transactions (sender_id, receiver_id, message, amount, time_stamp)
                                        VALUES(%s, %s, %s, %s, %s);"""
        
        #Insert users
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
            username_to_id[username] = user_id

            bank_account = user_data.get("bank_account", {})
            funds = bank_account["funds"]
            insert_funds_params = (user_id, funds)
            db_cursor.execute(insert_funds_sql, insert_funds_params)

            bank_account_id = db_cursor.fetchone()[0]
            username_to_bank_id[username] = bank_account_id

        #Insert bank transactions
        for user_data in users:
            username = user_data["username"]
            sender_id = username_to_bank_id[username]

            for bank_transaction in user_data["bank_transactions"]:
                receiver_username = bank_transaction["receiver"]
                receiver_id = username_to_bank_id.get(receiver_username)

                if receiver_id is None:
                    print(f"No receiver {receiver_username} bank account found!")
                    continue
                
                amount = bank_transaction["amount"]
                transaction_message = bank_transaction["message"]
                transaction_time_stamp = bank_transaction["time_stamp"]

                insert_bank_transaction_params = (sender_id, receiver_id, transaction_message, amount, transaction_time_stamp)

                db_cursor.execute(insert_bank_transaction_sql, insert_bank_transaction_params)


def main():
    parser = argparse.ArgumentParser(description = "Loads user data to the database from .json files")
    parser.add_argument("-u", help = "users .json file", required=True)
    parser.add_argument("-b", help = "banking .json file")
    parser.add_argument("-dc", help = "direct chats .json file")
    parser.add_argument("-gc", help = "group chats .json file")
    args = parser.parse_args()

    if not setup_configuration():
        return
   
   db_connection = psycopg2.connect(dbname = DATABASE_NAME,
                                         user = DATABASE_USERNAME,
                                         password = DATABASE_PASSWORD,
                                         host = DATABASE_URL,
                                         port = DATABASE_PORT);

    db_connection.autocommit = False

    if args.u:
        user_data_path = args.u
        
    if args.b:
        banking_data_path = args.b

    if args.dc:
        direct_chat_data_path = args.dc

    if args.gc:
        group_chat_data_path = args.gc
        
    
   




           
        #Direct chats
        for user_data in users:
            username = user_data["username"]
            sender_id = username_to_id[username]
            receivers = set()
            direct_messages = user_data["direct_messages"]

            for message in direct_messages:
                receiver = message["receiver"]
                receiver_id = username_to_id[receiver]
                
                min_id = min(sender_id, receiver_id)
                max_id = max(sender_id, receiver_id)
                
                insert_direct_chat_params = (min_id, max_id)
                db_cursor.execute(insert_direct_chat_sql, insert_direct_chat_params)
                direct_chat_id = db_cursor.fetchone()
                chat_user_pair = f"{min_id}+{max_id}"
                
                if direct_chat_id != None:
                    user_pair_to_direct_chat_id[chat_user_pair] = direct_chat_id[0]

                direct_chat_id = user_pair_to_direct_chat_id[chat_user_pair]               
                message_content = message["message"]
                time_stamp = message["time_stamp"]
                insert_direct_message_params = (direct_chat_id, sender_id, message_content, time_stamp)
                db_cursor.execute(insert_direct_message_sql, insert_direct_message_params)
        
        
        #Update last message and last message time stamp in direct chat
        update_last_message_and_time_stamp_sql = sql.SQL("""
            UPDATE direct_chats AS dc
            SET
                last_message = dm.message,
                last_message_time_stamp = dm.time_stamp
            FROM (
                SELECT DISTINCT ON (chat_id)
                    chat_id,
                    message,
                    time_stamp
                FROM direct_chat_messages
                ORDER BY chat_id, time_stamp DESC
            ) AS dm
            WHERE dc.id = dm.chat_id
        """)

        db_cursor.execute(update_last_message_and_time_stamp_sql)

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
