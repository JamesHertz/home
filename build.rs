use tokio::fs;
use sqlx::sqlite::SqlitePoolOptions;

static DB_BUILD_FILE      : &str = "sql/schema.db";
static DB_SQL_SCRIPT_FILE : &str = "sql/schema.sql";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed={DB_SQL_SCRIPT_FILE}");

    let _ = fs::remove_file(DB_BUILD_FILE).await;
    let sql_commands = fs::read_to_string(DB_SQL_SCRIPT_FILE).await?;
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&format!("sqlite:{DB_BUILD_FILE}?mode=rwc"))
        .await?;

    sqlx::raw_sql(&sql_commands).execute(&pool).await?;
    println!("cargo::rustc-env=DATABASE_URL=sqlite:sql/schema.db");
    Ok(())
}
