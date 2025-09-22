use sqlx::{postgres::PgPoolOptions, PgPool};

pub async fn connect_to_database(db_url: String) -> PgPool {
    let db_pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&db_url)
        .await;

    let db_pool = match db_pool {
        Ok(pool) => pool,
        Err(error) => {
            panic!("Error: Server did not connect to the database: {}", error);
        }
    };

    println!("Connected to postgres!");
    return db_pool;
}
