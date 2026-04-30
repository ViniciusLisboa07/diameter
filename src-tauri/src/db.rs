mod commands;
mod connection;
mod cover;
mod epub;
mod repository;
mod schema;
mod service;
mod types;

pub use schema::initialize_database;

use tauri::AppHandle;

#[tauri::command]
pub fn import_books(app: AppHandle, paths: Vec<String>) -> Result<types::ImportBooksResult, String> {
  commands::import_books(app, paths)
}

#[tauri::command]
pub fn delete_book(app: AppHandle, book_id: i64) -> Result<(), String> {
  commands::delete_book(app, book_id)
}

#[tauri::command]
pub fn update_book_metadata(app: AppHandle, payload: types::UpdateBookMetadataInput) -> Result<(), String> {
  commands::update_book_metadata(app, payload)
}

#[tauri::command]
pub async fn read_epub(app: AppHandle, book_id: i64) -> Result<types::EpubReadDto, String> {
  commands::read_epub(app, book_id).await
}

#[tauri::command]
pub fn save_reading_progress(
  app: AppHandle,
  book_id: i64,
  last_position: String,
  progress_percent: i64,
) -> Result<(), String> {
  commands::save_reading_progress(app, book_id, last_position, progress_percent)
}

#[tauri::command]
pub fn list_books(app: AppHandle) -> Result<Vec<types::BookDto>, String> {
  commands::list_books(app)
}
