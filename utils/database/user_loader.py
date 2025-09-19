import json
import argparse

def main():
    parser = argparse.ArgumentParser(description = "Loads user data to the database from .json file")
    parser.add_argument("-f", help = ".json file path")
    args = parser.parse_args()

    if not args.f:
        print("No .json file path provided")

    file_path = args.f

    with open(file_path, "r", encoding="utf-8") as file:
        users = json.load(file)

    for user in users:
        print("username: ", user["username"])
        print("password: ", user["password"])
        print("extra data: ", user["extra_data"])
        print("-" * 36)

if __name__ == "__main__":
    main()
