FROM python:3.12-slim
RUN pip install --no-cache-dir psycopg2-binary argon2-cffi
WORKDIR /app
COPY utils /app/utils
COPY docker/db_init.sh /app/db_init.sh
# The scripts read ../../server.conf, so run them from /app/utils/database.
WORKDIR /app/utils/database
CMD ["sh", "/app/db_init.sh"]
