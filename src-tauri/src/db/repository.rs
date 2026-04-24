use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection};

use super::types::{BookDto, UpdateBookMetadataInput};

pub fn insert_imported_book(
  conn: &Connection,
  title: &str,
  format: &str,
  file_path: &str,
  cover_image_data: Option<&str>,
) -> Result<(), String> {
  conn
    .execute(
      "INSERT INTO books (title, author, description, publication_year, cover_image_data) VALUES (?1, ?2, ?3, ?4, ?5)",
      params![title, "Autor desconhecido", "", 0_i64, cover_image_data],
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

pub fn list_book_file_paths(conn: &Connection, book_id: i64) -> Result<Vec<String>, String> {
  let mut stmt = conn
    .prepare(
      r#"
      SELECT file_path
      FROM book_formats
      WHERE book_id = ?1
        AND file_path IS NOT NULL
      "#,
    )
    .map_err(|err| format!("failed to prepare list_book_file_paths query: {err}"))?;

  let rows = stmt
    .query_map(params![book_id], |row| row.get::<_, String>(0))
    .map_err(|err| format!("failed to execute list_book_file_paths query: {err}"))?;

  let mut paths = Vec::new();
  for row in rows {
    paths.push(row.map_err(|err| format!("failed to parse list_book_file_paths row: {err}"))?);
  }

  Ok(paths)
}

pub fn list_books_missing_cover_sources(conn: &Connection) -> Result<Vec<(i64, String, String)>, String> {
  let mut stmt = conn
    .prepare(
      r#"
      SELECT
        b.id,
        (
          SELECT bf.format
          FROM book_formats bf
          WHERE bf.book_id = b.id
            AND bf.file_path IS NOT NULL
          ORDER BY bf.id ASC
          LIMIT 1
        ) AS format,
        (
          SELECT bf.file_path
          FROM book_formats bf
          WHERE bf.book_id = b.id
            AND bf.file_path IS NOT NULL
          ORDER BY bf.id ASC
          LIMIT 1
        ) AS file_path
      FROM books b
      WHERE (b.cover_image_data IS NULL OR b.cover_image_data = '')
        AND EXISTS (
          SELECT 1
          FROM book_formats bf2
          WHERE bf2.book_id = b.id
            AND bf2.file_path IS NOT NULL
            AND bf2.format IN ('EPUB', 'PDF')
        )
      ORDER BY b.id ASC
      "#,
    )
    .map_err(|err| format!("failed to prepare list_books_missing_cover_sources query: {err}"))?;

  let rows = stmt
    .query_map([], |row| {
      let book_id: i64 = row.get(0)?;
      let format: Option<String> = row.get(1)?;
      let file_path: Option<String> = row.get(2)?;
      Ok((book_id, format.unwrap_or_default(), file_path.unwrap_or_default()))
    })
    .map_err(|err| format!("failed to execute list_books_missing_cover_sources query: {err}"))?;

  let mut books = Vec::new();
  for row in rows {
    let (book_id, format, file_path) =
      row.map_err(|err| format!("failed to parse list_books_missing_cover_sources row: {err}"))?;

    if format.is_empty() || file_path.is_empty() {
      continue;
    }

    books.push((book_id, format, file_path));
  }

  Ok(books)
}

pub fn update_book_cover_image(conn: &Connection, book_id: i64, cover_image_data: &str) -> Result<(), String> {
  conn
    .execute(
      "UPDATE books SET cover_image_data = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
      params![cover_image_data, book_id],
    )
    .map_err(|err| format!("failed to update cover image for book {book_id}: {err}"))?;

  Ok(())
}

pub fn delete_book_by_id(conn: &Connection, book_id: i64) -> Result<(), String> {
  let rows_affected = conn
    .execute("DELETE FROM books WHERE id = ?1", params![book_id])
    .map_err(|err| format!("failed to delete book: {err}"))?;

  if rows_affected == 0 {
    return Err("book not found".to_string());
  }

  Ok(())
}

pub fn update_book_metadata(conn: &mut Connection, payload: UpdateBookMetadataInput) -> Result<(), String> {
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
    .execute("DELETE FROM book_tags WHERE book_id = ?1", params![payload.book_id])
    .map_err(|err| format!("failed to clear book tags: {err}"))?;

  for tag in tags {
    tx
      .execute(
        "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
        params![&tag],
      )
      .map_err(|err| format!("failed to upsert tag: {err}"))?;

    let tag_id: i64 = tx
      .query_row("SELECT id FROM tags WHERE name = ?1", params![&tag], |row| row.get(0))
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

pub fn fetch_epub_read_context(conn: &Connection, book_id: i64) -> Result<(String, String, Option<String>, i64), String> {
  conn
    .query_row(
      r#"
      SELECT b.title, bf.file_path, rp.last_position, COALESCE(rp.progress_percent, 0)
      FROM books b
      INNER JOIN book_formats bf ON bf.book_id = b.id
      LEFT JOIN reading_progress rp ON rp.book_id = b.id
      WHERE b.id = ?1
        AND bf.format = 'EPUB'
        AND bf.file_path IS NOT NULL
      ORDER BY bf.id ASC
      LIMIT 1
      "#,
      params![book_id],
      |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .map_err(|err| format!("failed to locate EPUB file path: {err}"))
}

pub fn upsert_reading_progress(
  conn: &Connection,
  book_id: i64,
  last_position: String,
  progress_percent: i64,
) -> Result<(), String> {
  let bounded_progress = progress_percent.clamp(0, 100);

  conn
    .execute(
      r#"
      INSERT INTO reading_progress (book_id, progress_percent, last_position, updated_at)
      VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
      ON CONFLICT(book_id) DO UPDATE
      SET progress_percent = excluded.progress_percent,
          last_position = excluded.last_position,
          updated_at = CURRENT_TIMESTAMP
      "#,
      params![book_id, bounded_progress, last_position],
    )
    .map_err(|err| format!("failed to save reading progress: {err}"))?;

  Ok(())
}

pub fn list_books(conn: &Connection) -> Result<Vec<BookDto>, String> {
  let mut stmt = conn
    .prepare(
      r#"
      SELECT
        b.id,
        b.title,
        b.author,
        b.description,
        b.cover_image_data,
        b.publication_year,
        COALESCE((
          SELECT bf.format
          FROM book_formats bf
          WHERE bf.book_id = b.id
          ORDER BY bf.id ASC
          LIMIT 1
        ), 'UNKNOWN') AS format,
        COALESCE(rp.progress_percent, 0) AS progress,
        EXISTS(
          SELECT 1
          FROM book_formats bf2
          WHERE bf2.book_id = b.id
            AND bf2.format = 'EPUB'
            AND bf2.file_path IS NOT NULL
        ) AS is_epub_available
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
        cover_image_data: row.get(4)?,
        year: row.get(5)?,
        format: row.get(6)?,
        progress: row.get(7)?,
        tags: Vec::new(),
        is_epub_available: row.get(8)?,
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

fn normalize_tags(tags: &[String]) -> Vec<String> {
  let mut deduped = Vec::new();
  let mut seen = HashSet::new();

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
