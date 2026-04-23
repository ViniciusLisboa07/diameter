use std::{fs, path::PathBuf};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
  let app_data_dir = app
    .path()
    .app_local_data_dir()
    .map_err(|err| format!("failed to get app local data dir: {err}"))?;

  fs::create_dir_all(&app_data_dir).map_err(|err| format!("failed to create app data dir: {err}"))?;

  Ok(app_data_dir.join("diameter.sqlite3"))
}

pub fn open_connection(app: &AppHandle) -> Result<Connection, String> {
  let path = db_path(app)?;
  Connection::open(path).map_err(|err| format!("failed to open sqlite database: {err}"))
}

pub fn library_dir(app: &AppHandle) -> Result<PathBuf, String> {
  let app_data_dir = app
    .path()
    .app_local_data_dir()
    .map_err(|err| format!("failed to get app local data dir: {err}"))?;

  let library = app_data_dir.join("library");
  fs::create_dir_all(&library).map_err(|err| format!("failed to create library dir: {err}"))?;

  Ok(library)
}
