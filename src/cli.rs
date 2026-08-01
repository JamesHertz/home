use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng}
};

use sqlx::sqlite::SqlitePoolOptions;
use tokio::fs;
use std::process;

pub async fn init_app(admin_password : &str, db_file : &str) -> crate::server::Result<()> {
    if fs::try_exists(db_file).await? {
        tracing::error!("DB file '{db_file}' already exists!!!");
        process::exit(1)
    }

    let sql_commands = fs::read_to_string("./sql/schema.sql").await?;
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite:{db_file}?mode=rwc"))
        .await?;


    sqlx::raw_sql(&sql_commands).execute(&pool).await?;
    let password = admin_password.as_bytes();
    let salt = SaltString::generate(&mut OsRng);

    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password, &salt)
        .unwrap()
        .to_string();

    sqlx::query("INSERT INTO Users(username, login, pwd_hash) VALUES (?, ?, ?)")
        .bind("Super Cool Admin")
        .bind("admin")
        .bind(password_hash)
        .execute(&pool)
        .await?;

    Ok(())
}
