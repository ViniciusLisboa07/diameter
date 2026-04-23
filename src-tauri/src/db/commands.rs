use tauri::AppHandle;

use super::{
  connection::open_connection,
  repository,
  service,
  types::{BookDto, EpubReadDto, ImportBooksResult, UpdateBookMetadataInput},
};

pub fn import_books(app: AppHandle, paths: Vec<String>) -> Result<ImportBooksResult, String> {
  service::import_books(app, paths)
}

pub fn update_book_metadata(app: AppHandle, payload: UpdateBookMetadataInput) -> Result<(), String> {
  let mut conn = open_connection(&app)?;
  repository::update_book_metadata(&mut conn, payload)
}

pub fn read_epub(app: AppHandle, book_id: i64) -> Result<EpubReadDto, String> {
  service::read_epub(app, book_id)
}

pub fn save_reading_progress(
  app: AppHandle,
  book_id: i64,
  last_position: String,
  progress_percent: i64,
) -> Result<(), String> {
  let conn = open_connection(&app)?;
  repository::upsert_reading_progress(&conn, book_id, last_position, progress_percent)
}

pub fn list_books(app: AppHandle) -> Result<Vec<BookDto>, String> {
  let conn = open_connection(&app)?;
  repository::list_books(&conn)
}
