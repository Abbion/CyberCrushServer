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
MAX_GROUP_CHAT_MEMBERS = 16

DATABASE_NAME = ""
DATABASE_USERNAME = ""
DATABASE_PASSWORD = ""
DATABASE_URL = ""
DATABASE_PORT = 0

def hash_password(password: str, ph: PasswordHasher) -> str:
    return ph.hash(password + PEPPER)

def setup_configuration():
    global PEPPER, MAX_UESRNAME_LENGTH, MAX_PASSWORD_LENGTH, MAX_EXTRA_DATA_LENGTH
    global DATABASE_NAME, DATABASE_USERNAME, DATABASE_PASSWORD, DATABASE_URL, DATABASE_PORT
    
    with open("../../server.conf", "r", encoding="utf-8") as file:
        config = json.load(file)

    PEPPER = config["database_password_pepper"]
    MAX_USERNAME_LENGTH = config["max_username_length"]
    MAX_PASSWORD_LENGTH = config["max_password_length"]
    MAX_EXTRA_DATA_LENGTH = config["max_extra_data_length"]
    MAX_GROUP_CHAT_MEMVERS = config["max_group_chat_members"]

    DATABASE_NAME = config["database_name"]
    DATABASE_USERNAME = config["database_admin_username"]
    DATABASE_PASSWORD = config["database_admin_password"]
    DATABASE_URL = config["database_url"]
    DATABASE_PORT = config["database_port"]

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

    username_to_id = {}

    try:
        db_cursor = db_connection.cursor()

        insert_user_sql = """INSERT INTO users (username, password, user_token, personal_number, extra_data)
                            VALUES (%s, %s, %s, %s, %s) RETURNING id;"""
        
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

    except Exception:
        if db_connection:
            db_connection.rollback()
        raise
    finally:
        if db_connection:
            db_cursor.close()

    return username_to_id

def load_banking_data(banking_data_path, username_to_id_map, db_connection):
    with open(banking_data_path, "r", encoding="utf-8") as file:
        bank_accounts = json.load(file)
    
    username_to_bank_id = {}

    try:
        db_cursor = db_connection.cursor()

        insert_funds_sql = """INSERT INTO bank_accounts (user_id, funds)
                                VALUES (%s, %s) RETURNING id;"""
        insert_bank_transaction_sql = """INSERT INTO bank_transactions (sender_id, receiver_id, message, amount, time_stamp)
                                        VALUES(%s, %s, %s, %s, %s);"""

        #Insert accounts with funds
        for account in bank_accounts:
            account_owner_username = account["username"]
            account_owner_id = username_to_id_map[account_owner_username]
            current_funds = account["current_funds"]

            insert_funds_params = (account_owner_id, current_funds)
            db_cursor.execute(insert_funds_sql, insert_funds_params)

            bank_account_id = db_cursor.fetchone()[0]
            username_to_bank_id[account_owner_username] = bank_account_id

        #Insert transactions
        for account in bank_accounts:
            sender_username = account["username"]
            sender_id = username_to_bank_id[sender_username]
            transactions = account["transactions"]

            for transaction in transactions:
                receiver_username = transaction["receiver"]
                receiver_id = username_to_bank_id[receiver_username]

                if receiver_id is None:
                    print(f"No receiver {receiver_username} bank account found!")
                    continue
                
                message = transaction["message"]
                amount = transaction["amount"]
                time_stamp = transaction["time_stamp"]

                insert_bank_transaction_params = (sender_id, receiver_id, message, amount, time_stamp)
                db_cursor.execute(insert_bank_transaction_sql, insert_bank_transaction_params)

    except Exception:
        if db_connection:
            db_connection.rollback()
        raise
    finally:
        if db_connection:
            db_cursor.close()

def insert_chat(db_cursor):
    insert_chat_sql = """INSERT INTO chats DEFAULT VALUES RETURNING id;"""
    db_cursor.execute(insert_chat_sql)
    return db_cursor.fetchone()[0]

def insert_chat_message(db_cursor, chat_id, in_chat_index, sender_id, content, time_stamp):
    insert_message_sql = """INSERT INTO chat_messages (chat_id, in_chat_index, sender_id, content, time_stamp)
                                VALUES (%s, %s, %s, %s, %s);"""
    insert_message_params = (chat_id, in_chat_index, sender_id, content, time_stamp)
    db_cursor.execute(insert_message_sql, insert_message_params)

def assign_user_chat(db_cursor, chat_id, user_id):
    insert_user_chat_sql = """INSERT INTO user_chats (chat_id, user_id)
                                VALUES (%s, %s);"""
    insert_user_chat_params = (chat_id, user_id)
    db_cursor.execute(insert_user_chat_sql, insert_user_chat_params)

def get_last_message_and_time_stamp(db_cursor, chat_id):
    get_last_message_info_sql = """SELECT content, time_stamp
                                        FROM chat_messages
                                        WHERE chat_id = %s
                                        ORDER BY time_stamp DESC
                                        LIMIT 1;"""

    db_cursor.execute(get_last_message_info_sql, (chat_id,))
    return db_cursor.fetchone()
     

def load_direct_chat_data(direct_chat_data_path, username_to_id_map, db_connection):
    with open(direct_chat_data_path, "r", encoding="utf-8") as file:
        direct_chats = json.load(file)

    try:
        db_cursor = db_connection.cursor()
        
        insert_direct_chat_sql = """INSERT INTO direct_chats (chat_id, next_message_index, last_message, last_time_stamp)
                                        VALUES (%s, %s, %s, %s);"""
       
        for direct_chat in direct_chats:
            user_a = direct_chat["user_a"]
            user_a_id = username_to_id_map[user_a]
            user_b = direct_chat["user_b"]
            user_b_id = username_to_id_map[user_b]
            messages = direct_chat["messages"]

            chat_id = insert_chat(db_cursor)
            assign_user_chat(db_cursor, chat_id, user_a_id)
            assign_user_chat(db_cursor, chat_id, user_b_id)

            for index, message in enumerate(messages):
                sender = message["sender"]
                if sender == "a":
                    sender = user_a_id
                elif sender == "b":
                    sender = user_b_id
                else:
                    print("Sender tag is not a or b in direct_chat: ", direct_chat)
                    continue
                
                content = message["content"]
                time_stamp = message["time_stamp"]

                insert_chat_message(db_cursor, chat_id, index, sender, content, time_stamp)

            last_message_info = get_last_message_and_time_stamp(db_cursor, chat_id)

            if last_message_info:
                last_message, last_time_stamp = last_message_info
                insert_direct_chat_params = (chat_id, len(messages), last_message, last_time_stamp)
                db_cursor.execute(insert_direct_chat_sql, insert_direct_chat_params)

 
    except Exception:
        if db_connection:
            db_connection.rollback()
        raise
    finally:
        if db_connection:
            db_cursor.close()

def load_group_chat_data(group_chat_data_path, username_to_id_map, db_connection):
    with open(group_chat_data_path, "r", encoding="utf-8") as file:
        group_chats = json.load(file)

    try:
        db_cursor = db_connection.cursor()

        insert_group_chat_sql = """INSERT INTO group_chats (chat_id, admin_id, next_message_index, title, last_message, last_time_stamp)
                                        VALUES (%s, %s, %s, %s, %s, %s);"""

        for group_chat in group_chats:
            admin = group_chat["admin"]
            admin_id = username_to_id_map[admin]
            title = group_chat["title"]
            members = {}

            chat_id = insert_chat(db_cursor)

            for i in range(1, MAX_GROUP_CHAT_MEMBERS + 1):
                member_tag = f"user_{i}"
                member_username = group_chat.get(member_tag)
                if member_username == None:
                    break
                member_id = username_to_id_map[member_username]
                members[i] = member_id
                assign_user_chat(db_cursor, chat_id, member_id)
            messages = group_chat["messages"]

            for index, message in enumerate(messages):
                sender_tag = message["sender"]
                content = message["content"]
                time_stamp = message["time_stamp"]
                sender_id = members[sender_tag]
                
                insert_chat_message(db_cursor, chat_id, index, sender_id, content, time_stamp)
            
            last_message_info = get_last_message_and_time_stamp(db_cursor, chat_id)

            if last_message_info:
                last_message, last_time_stamp = last_message_info
                insert_group_chat_params = (chat_id, admin_id, len(messages), title, last_message, last_time_stamp)
                db_cursor.execute(insert_group_chat_sql, insert_group_chat_params)
             

    except Exception:
        if db_connection:
            db_connection.rollback()
        raise
    finally:
        if db_connection:
            db_cursor.close()

def load_news_articles_data(news_articles_data_path, username_to_id_map, db_connection):
    with open(news_articles_data_path, "r", encoding="utf-8") as file:
        news_articles = json.load(file)

    try:
        db_cursor = db_connection.cursor()

        insert_news_article_sql = """INSERT INTO news_articles (user_id, title, content, timestamp)
                                        VALUES (%s, %s, %s, %s);"""
        for article in news_articles:
            poster_username = article["username"]
            user_id = username_to_id_map[poster_username]
            title = article["title"]
            content = article["content"]
            timestamp = article["timestamp"]

            insert_news_article_params = (user_id, title, content, timestamp)
            db_cursor.execute(insert_news_article_sql, insert_news_article_params)

    except Exception:
        if db_connection:
            db_connection.rollback()
        raise
    finally:
        if db_connection:
            db_cursor.close()

def main():
    parser = argparse.ArgumentParser(description = "Loads user data to the database from .json files")
    parser.add_argument("-u", help = "users .json file", required=True)
    parser.add_argument("-b", help = "banking .json file")
    parser.add_argument("-dc", help = "direct chats .json file")
    parser.add_argument("-gc", help = "group chats .json file")
    parser.add_argument("-na", help = "news articles .json file")
    args = parser.parse_args()

    setup_configuration()
   
    db_connection = psycopg2.connect(dbname = DATABASE_NAME,
                                    user = DATABASE_USERNAME,
                                    password = DATABASE_PASSWORD,
                                    host = DATABASE_URL,
                                    port = DATABASE_PORT);

    db_connection.autocommit = False
    username_to_id_map = {}

    if args.u:
        print("Loading user data...")
        user_data_path = args.u
        username_to_id_map = load_user_data(user_data_path, db_connection)

    if args.b:
        print("Loading banking data...")
        banking_data_path = args.b
        load_banking_data(banking_data_path, username_to_id_map, db_connection)

    if args.dc:
        print("Loading direct chat data...")
        direct_chat_data_path = args.dc
        load_direct_chat_data(direct_chat_data_path, username_to_id_map, db_connection)

    if args.gc:
        print("Loading group chat data...")
        group_chat_data_path = args.gc
        load_group_chat_data(group_chat_data_path, username_to_id_map, db_connection)

    if args.na:
        print("Loading news articles data...")
        news_articles_data_path = args.na
        load_news_articles_data(news_articles_data_path, username_to_id_map, db_connection)
    
    db_connection.commit()
    db_connection.close()
    print("Loader finished")
    
if __name__ == "__main__":
    main()
