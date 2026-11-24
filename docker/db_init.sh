#!/bin/sh
set -e

python3 setup_database.py

if [ "$LOAD_EXAMPLE_DATA" = "true" ]; then
    already_loaded=$(python3 - <<'PY'
import json, psycopg2
c = json.load(open("../../server.conf"))
conn = psycopg2.connect(dbname=c["database_name"], user=c["database_admin_username"],
                        password=c["database_admin_password"], host=c["database_url"], port=c["database_port"])
cur = conn.cursor()
cur.execute("SELECT COUNT(*) FROM users")
print(cur.fetchone()[0])
PY
)
    if [ "$already_loaded" = "0" ]; then
        d=example_data
        python3 user_loader.py -u $d/example_users.json -b $d/example_banking.json \
            -dc $d/example_direct_chats.json -gc $d/example_group_chats.json -na $d/example_articles.json
    else
        echo "Users table is not empty, skipping example data."
    fi
fi
