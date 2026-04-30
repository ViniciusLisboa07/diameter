mod db;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      let app_handle = app.handle().clone();
      if let Err(err) = db::initialize_database(&app_handle) {
                return Err(std::io::Error::other(format!(
                    "database initialization failed: {err}"
                ))
                .into());
      }

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      db::list_books,
      db::import_books,
      db::delete_book,
      db::update_book_metadata,
      db::read_epub,
      db::save_reading_progress
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
