use rusqlite::{params, Connection};
use tauri::AppHandle;

use super::connection::open_connection;

pub fn initialize_database(app: &AppHandle) -> Result<(), String> {
  let conn = open_connection(app)?;

    conn.execute_batch(
      r#"
    PRAGMA foreign_keys = ON;

    CREATE TABLE IF NOT EXISTS books (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      title TEXT NOT NULL,
      author TEXT NOT NULL,
      description TEXT NOT NULL DEFAULT '',
      cover_image_data TEXT,
      publication_year INTEGER NOT NULL,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );

    CREATE TABLE IF NOT EXISTS book_formats (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      book_id INTEGER NOT NULL,
      format TEXT NOT NULL,
      file_path TEXT,
      created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY(book_id) REFERENCES books(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS tags (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      name TEXT NOT NULL UNIQUE
    );

    CREATE TABLE IF NOT EXISTS book_tags (
      book_id INTEGER NOT NULL,
      tag_id INTEGER NOT NULL,
      PRIMARY KEY(book_id, tag_id),
      FOREIGN KEY(book_id) REFERENCES books(id) ON DELETE CASCADE,
      FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS reading_progress (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      book_id INTEGER NOT NULL UNIQUE,
      progress_percent INTEGER NOT NULL DEFAULT 0,
      last_position TEXT,
      updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
      FOREIGN KEY(book_id) REFERENCES books(id) ON DELETE CASCADE
    );
    "#,
    )
    .map_err(|err| format!("failed to run schema migration: {err}"))?;

  ensure_optional_columns(&conn)?;

  let books_count: i64 = conn
    .query_row("SELECT COUNT(1) FROM books", [], |row| row.get(0))
    .map_err(|err| format!("failed to count books: {err}"))?;

  if books_count == 0 {
    seed_database(&conn)?;
  }

  Ok(())
}

fn ensure_optional_columns(conn: &Connection) -> Result<(), String> {
  if !has_column(conn, "books", "cover_image_data")? {
        conn.execute("ALTER TABLE books ADD COLUMN cover_image_data TEXT", [])
      .map_err(|err| format!("failed to add books.cover_image_data column: {err}"))?;
  }

  Ok(())
}

fn has_column(conn: &Connection, table_name: &str, column_name: &str) -> Result<bool, String> {
  let query = format!("PRAGMA table_info({table_name})");

  let mut stmt = conn
    .prepare(&query)
    .map_err(|err| format!("failed to prepare table_info for {table_name}: {err}"))?;

  let rows = stmt
    .query_map([], |row| row.get::<_, String>(1))
    .map_err(|err| format!("failed to inspect columns for {table_name}: {err}"))?;

  for row in rows {
    let name = row.map_err(|err| format!("failed to parse table_info row: {err}"))?;
    if name == column_name {
      return Ok(true);
    }
  }

  Ok(false)
}

fn seed_database(conn: &Connection) -> Result<(), String> {
  conn
    .execute(
      "INSERT INTO books (title, author, description, publication_year) VALUES (?1, ?2, ?3, ?4)",
      params![
        "The Pragmatic Programmer",
        "Andrew Hunt & David Thomas",
        "Guia prático sobre decisões de engenharia de software, qualidade de código e evolução incremental de sistemas.",
        2019_i64
      ],
    )
    .map_err(|err| format!("failed to insert mock book 1: {err}"))?;
  let book_1_id = conn.last_insert_rowid();

  conn
    .execute(
      "INSERT INTO books (title, author, description, publication_year) VALUES (?1, ?2, ?3, ?4)",
      params![
        "Designing Data-Intensive Applications",
        "Martin Kleppmann",
        "Referência para arquiteturas modernas com foco em bancos de dados, consistência, escalabilidade e processamento de dados.",
        2017_i64
      ],
    )
    .map_err(|err| format!("failed to insert mock book 2: {err}"))?;
  let book_2_id = conn.last_insert_rowid();

  conn
    .execute(
      "INSERT INTO books (title, author, description, publication_year) VALUES (?1, ?2, ?3, ?4)",
      params![
        "Clean Architecture",
        "Robert C. Martin",
        "Aborda organização de código e separação de responsabilidades para manter sistemas sustentáveis no longo prazo.",
        2018_i64
      ],
    )
    .map_err(|err| format!("failed to insert mock book 3: {err}"))?;
  let book_3_id = conn.last_insert_rowid();

    conn.execute(
      "INSERT INTO book_formats (book_id, format, file_path) VALUES (?1, ?2, ?3)",
      params![book_1_id, "EPUB", Option::<String>::None],
    )
    .map_err(|err| format!("failed to insert format for book 1: {err}"))?;

    conn.execute(
      "INSERT INTO book_formats (book_id, format, file_path) VALUES (?1, ?2, ?3)",
      params![book_2_id, "PDF", Option::<String>::None],
    )
    .map_err(|err| format!("failed to insert format for book 2: {err}"))?;

    conn.execute(
      "INSERT INTO book_formats (book_id, format, file_path) VALUES (?1, ?2, ?3)",
      params![book_3_id, "EPUB", Option::<String>::None],
    )
    .map_err(|err| format!("failed to insert format for book 3: {err}"))?;

    conn.execute("INSERT INTO tags (name) VALUES ('engenharia')", [])
    .map_err(|err| format!("failed to insert tag engenharia: {err}"))?;
  let tag_engenharia = conn.last_insert_rowid();

    conn.execute("INSERT INTO tags (name) VALUES ('boas práticas')", [])
    .map_err(|err| format!("failed to insert tag boas práticas: {err}"))?;
  let tag_boas_praticas = conn.last_insert_rowid();

    conn.execute("INSERT INTO tags (name) VALUES ('arquitetura')", [])
    .map_err(|err| format!("failed to insert tag arquitetura: {err}"))?;
  let tag_arquitetura = conn.last_insert_rowid();

    conn.execute("INSERT INTO tags (name) VALUES ('dados')", [])
    .map_err(|err| format!("failed to insert tag dados: {err}"))?;
  let tag_dados = conn.last_insert_rowid();

    conn.execute("INSERT INTO tags (name) VALUES ('clean code')", [])
    .map_err(|err| format!("failed to insert tag clean code: {err}"))?;
  let tag_clean_code = conn.last_insert_rowid();

    conn.execute(
      "INSERT INTO book_tags (book_id, tag_id) VALUES (?1, ?2)",
      params![book_1_id, tag_engenharia],
    )
    .map_err(|err| format!("failed to link tag engenharia to book 1: {err}"))?;

    conn.execute(
      "INSERT INTO book_tags (book_id, tag_id) VALUES (?1, ?2)",
      params![book_1_id, tag_boas_praticas],
    )
    .map_err(|err| format!("failed to link tag boas práticas to book 1: {err}"))?;

    conn.execute(
      "INSERT INTO book_tags (book_id, tag_id) VALUES (?1, ?2)",
      params![book_2_id, tag_arquitetura],
    )
    .map_err(|err| format!("failed to link tag arquitetura to book 2: {err}"))?;

    conn.execute(
      "INSERT INTO book_tags (book_id, tag_id) VALUES (?1, ?2)",
      params![book_2_id, tag_dados],
    )
    .map_err(|err| format!("failed to link tag dados to book 2: {err}"))?;

    conn.execute(
      "INSERT INTO book_tags (book_id, tag_id) VALUES (?1, ?2)",
      params![book_3_id, tag_arquitetura],
    )
    .map_err(|err| format!("failed to link tag arquitetura to book 3: {err}"))?;

    conn.execute(
      "INSERT INTO book_tags (book_id, tag_id) VALUES (?1, ?2)",
      params![book_3_id, tag_clean_code],
    )
    .map_err(|err| format!("failed to link tag clean code to book 3: {err}"))?;

  conn
    .execute(
      "INSERT INTO reading_progress (book_id, progress_percent, last_position) VALUES (?1, ?2, ?3)",
      params![book_1_id, 38_i64, "chapter-04"],
    )
    .map_err(|err| format!("failed to insert reading progress for book 1: {err}"))?;

  conn
    .execute(
      "INSERT INTO reading_progress (book_id, progress_percent, last_position) VALUES (?1, ?2, ?3)",
      params![book_2_id, 12_i64, "page-56"],
    )
    .map_err(|err| format!("failed to insert reading progress for book 2: {err}"))?;

  conn
    .execute(
      "INSERT INTO reading_progress (book_id, progress_percent, last_position) VALUES (?1, ?2, ?3)",
      params![book_3_id, 71_i64, "chapter-09"],
    )
    .map_err(|err| format!("failed to insert reading progress for book 3: {err}"))?;

  Ok(())
}
