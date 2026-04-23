use std::{
  collections::HashMap,
  ffi::OsStr,
  fs,
  path::{Path, PathBuf},
};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookDto {
  pub id: i64,
  pub title: String,
  pub author: String,
  pub description: String,
  pub format: String,
  pub year: i64,
  pub progress: i64,
  pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRejection {
  pub file_name: String,
  pub reason: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportBooksResult {
  pub imported_count: usize,
  pub rejected: Vec<ImportRejection>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBookMetadataInput {
  pub book_id: i64,
  pub title: String,
  pub author: String,
  pub description: String,
  pub tags: Vec<String>,
}

fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
  let app_data_dir = app
    .path()
    .app_local_data_dir()
    .map_err(|err| format!("failed to get app local data dir: {err}"))?;

  fs::create_dir_all(&app_data_dir).map_err(|err| format!("failed to create app data dir: {err}"))?;

  Ok(app_data_dir.join("diameter.sqlite3"))
}

fn open_connection(app: &AppHandle) -> Result<Connection, String> {
  let path = db_path(app)?;

  Connection::open(path).map_err(|err| format!("failed to open sqlite database: {err}"))
}

fn library_dir(app: &AppHandle) -> Result<PathBuf, String> {
  let app_data_dir = app
    .path()
    .app_local_data_dir()
    .map_err(|err| format!("failed to get app local data dir: {err}"))?;

  let library = app_data_dir.join("library");
  fs::create_dir_all(&library).map_err(|err| format!("failed to create library dir: {err}"))?;

  Ok(library)
}

fn normalize_book_title(file_path: &Path) -> String {
  let stem = file_path
    .file_stem()
    .and_then(OsStr::to_str)
    .unwrap_or("Livro sem título")
    .trim();

  let normalized = stem.replace(['_', '-'], " ").trim().to_owned();

  if normalized.is_empty() {
    "Livro sem título".to_string()
  } else {
    normalized
  }
}

fn resolve_format(file_path: &Path) -> Option<&'static str> {
  let ext = file_path.extension().and_then(OsStr::to_str)?.to_lowercase();

  match ext.as_str() {
    "epub" => Some("EPUB"),
    "pdf" => Some("PDF"),
    _ => None,
  }
}

fn next_available_destination(source: &Path, destination_dir: &Path) -> PathBuf {
  let stem = source
    .file_stem()
    .and_then(OsStr::to_str)
    .unwrap_or("arquivo")
    .to_string();
  let ext = source
    .extension()
    .and_then(OsStr::to_str)
    .map(|value| format!(".{value}"))
    .unwrap_or_default();

  let mut counter = 0_u32;

  loop {
    let file_name = if counter == 0 {
      format!("{stem}{ext}")
    } else {
      format!("{stem}-{counter}{ext}")
    };

    let candidate = destination_dir.join(file_name);

    if !candidate.exists() {
      return candidate;
    }

    counter += 1;
  }
}

fn insert_imported_book(conn: &Connection, title: &str, format: &str, file_path: &str) -> Result<(), String> {
  conn
    .execute(
      "INSERT INTO books (title, author, description, publication_year) VALUES (?1, ?2, ?3, ?4)",
      params![title, "Autor desconhecido", "", 0_i64],
    )
    .map_err(|err| format!("failed to create imported book: {err}"))?;

  let book_id = conn.last_insert_rowid();

  conn
    .execute(
      "INSERT INTO book_formats (book_id, format, file_path) VALUES (?1, ?2, ?3)",
      params![book_id, format, file_path],
    )
    .map_err(|err| format!("failed to create imported format: {err}"))?;

  conn
    .execute(
      "INSERT INTO reading_progress (book_id, progress_percent, last_position) VALUES (?1, ?2, ?3)",
      params![book_id, 0_i64, Option::<String>::None],
    )
    .map_err(|err| format!("failed to create imported reading progress: {err}"))?;

  Ok(())
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
  let mut deduped = Vec::new();
  let mut seen = std::collections::HashSet::new();

  for tag in tags {
    let normalized = tag.trim();
    if normalized.is_empty() {
      continue;
    }

    let dedupe_key = normalized.to_lowercase();
    if seen.insert(dedupe_key) {
      deduped.push(normalized.to_string());
    }
  }

  deduped
}

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

  let books_count: i64 = conn
    .query_row("SELECT COUNT(1) FROM books", [], |row| row.get(0))
    .map_err(|err| format!("failed to count books: {err}"))?;

  if books_count == 0 {
    seed_database(&conn)?;
  }

  Ok(())
}

fn seed_database(conn: &Connection) -> Result<(), String> {
  conn.execute(
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

  conn.execute(
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

  conn.execute(
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

  conn.execute(
    "INSERT INTO reading_progress (book_id, progress_percent, last_position) VALUES (?1, ?2, ?3)",
    params![book_1_id, 38_i64, "chapter-04"],
  )
  .map_err(|err| format!("failed to insert reading progress for book 1: {err}"))?;

  conn.execute(
    "INSERT INTO reading_progress (book_id, progress_percent, last_position) VALUES (?1, ?2, ?3)",
    params![book_2_id, 12_i64, "page-56"],
  )
  .map_err(|err| format!("failed to insert reading progress for book 2: {err}"))?;

  conn.execute(
    "INSERT INTO reading_progress (book_id, progress_percent, last_position) VALUES (?1, ?2, ?3)",
    params![book_3_id, 71_i64, "chapter-09"],
  )
  .map_err(|err| format!("failed to insert reading progress for book 3: {err}"))?;

  Ok(())
}

#[tauri::command]
pub fn import_books(app: AppHandle, paths: Vec<String>) -> Result<ImportBooksResult, String> {
  let conn = open_connection(&app)?;
  let library = library_dir(&app)?;

  let mut imported_count = 0_usize;
  let mut rejected = Vec::new();

  for raw_path in paths {
    let source_path = PathBuf::from(&raw_path);
    let file_name = source_path
      .file_name()
      .and_then(OsStr::to_str)
      .unwrap_or("arquivo")
      .to_string();

    if !source_path.exists() || !source_path.is_file() {
      rejected.push(ImportRejection {
        file_name,
        reason: "arquivo não encontrado".to_string(),
      });
      continue;
    }

    let Some(format) = resolve_format(&source_path) else {
      rejected.push(ImportRejection {
        file_name,
        reason: "extensão não suportada (use EPUB ou PDF)".to_string(),
      });
      continue;
    };

    let destination = next_available_destination(&source_path, &library);

    if let Err(err) = fs::copy(&source_path, &destination) {
      rejected.push(ImportRejection {
        file_name,
        reason: format!("falha ao copiar arquivo: {err}"),
      });
      continue;
    }

    let destination_text = destination.to_string_lossy().to_string();

    if let Err(err) = insert_imported_book(&conn, &normalize_book_title(&source_path), format, &destination_text) {
      let _ = fs::remove_file(&destination);
      rejected.push(ImportRejection {
        file_name,
        reason: err,
      });
      continue;
    }

    imported_count += 1;
  }

  Ok(ImportBooksResult {
    imported_count,
    rejected,
  })
}

#[tauri::command]
pub fn update_book_metadata(app: AppHandle, payload: UpdateBookMetadataInput) -> Result<(), String> {
  let mut conn = open_connection(&app)?;
  let tx = conn
    .transaction()
    .map_err(|err| format!("failed to start update_book_metadata transaction: {err}"))?;

  let title = payload.title.trim();
  let author = payload.author.trim();
  let description = payload.description.trim();
  let tags = normalize_tags(&payload.tags);

  let rows_affected = tx
    .execute(
      r#"
      UPDATE books
      SET title = ?1, author = ?2, description = ?3, updated_at = CURRENT_TIMESTAMP
      WHERE id = ?4
      "#,
      params![
        if title.is_empty() { "Livro sem título" } else { title },
        if author.is_empty() {
          "Autor desconhecido"
        } else {
          author
        },
        description,
        payload.book_id
      ],
    )
    .map_err(|err| format!("failed to update book metadata: {err}"))?;

  if rows_affected == 0 {
    return Err("book not found".to_string());
  }

  tx
    .execute(
      "DELETE FROM book_tags WHERE book_id = ?1",
      params![payload.book_id],
    )
    .map_err(|err| format!("failed to clear book tags: {err}"))?;

  for tag in tags {
    tx
      .execute(
        "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
        params![tag],
      )
      .map_err(|err| format!("failed to upsert tag: {err}"))?;

    let tag_id: i64 = tx
      .query_row("SELECT id FROM tags WHERE name = ?1", params![tag], |row| row.get(0))
      .map_err(|err| format!("failed to fetch tag id: {err}"))?;

    tx
      .execute(
        "INSERT INTO book_tags (book_id, tag_id) VALUES (?1, ?2)",
        params![payload.book_id, tag_id],
      )
      .map_err(|err| format!("failed to assign tag to book: {err}"))?;
  }

  tx
    .commit()
    .map_err(|err| format!("failed to commit update_book_metadata transaction: {err}"))?;

  Ok(())
}

#[tauri::command]
pub fn list_books(app: AppHandle) -> Result<Vec<BookDto>, String> {
  let conn = open_connection(&app)?;

  let mut stmt = conn
    .prepare(
      r#"
      SELECT
        b.id,
        b.title,
        b.author,
        b.description,
        b.publication_year,
        COALESCE((
          SELECT bf.format
          FROM book_formats bf
          WHERE bf.book_id = b.id
          ORDER BY bf.id ASC
          LIMIT 1
        ), 'UNKNOWN') AS format,
        COALESCE(rp.progress_percent, 0) AS progress
      FROM books b
      LEFT JOIN reading_progress rp ON rp.book_id = b.id
      ORDER BY b.created_at DESC, b.id DESC
      "#,
    )
    .map_err(|err| format!("failed to prepare list_books query: {err}"))?;

  let books_iter = stmt
    .query_map([], |row| {
      Ok(BookDto {
        id: row.get(0)?,
        title: row.get(1)?,
        author: row.get(2)?,
        description: row.get(3)?,
        year: row.get(4)?,
        format: row.get(5)?,
        progress: row.get(6)?,
        tags: Vec::new(),
      })
    })
    .map_err(|err| format!("failed to execute list_books query: {err}"))?;

  let mut books = Vec::new();
  for book_row in books_iter {
    books.push(book_row.map_err(|err| format!("failed to parse list_books row: {err}"))?);
  }

  let mut tags_by_book_id: HashMap<i64, Vec<String>> = HashMap::new();
  let mut tags_stmt = conn
    .prepare(
      r#"
      SELECT bt.book_id, t.name
      FROM book_tags bt
      INNER JOIN tags t ON t.id = bt.tag_id
      ORDER BY bt.book_id ASC, t.name ASC
      "#,
    )
    .map_err(|err| format!("failed to prepare tags query: {err}"))?;

  let tags_iter = tags_stmt
    .query_map([], |row| {
      let book_id: i64 = row.get(0)?;
      let tag: String = row.get(1)?;
      Ok((book_id, tag))
    })
    .map_err(|err| format!("failed to execute tags query: {err}"))?;

  for tag_row in tags_iter {
    let (book_id, tag) = tag_row.map_err(|err| format!("failed to parse tags row: {err}"))?;
    tags_by_book_id.entry(book_id).or_default().push(tag);
  }

  for book in &mut books {
    book.tags = tags_by_book_id.remove(&book.id).unwrap_or_default();
  }

  Ok(books)
}
