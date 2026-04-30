use std::{
  ffi::OsStr,
  fs,
  path::{Path, PathBuf},
  time::Instant,
};

use tauri::AppHandle;

use super::{
  connection::{library_dir, open_connection},
  cover::extract_cover_image_data_uri,
  epub::{parse_last_chapter_index, read_epub_file},
  repository,
  types::{BookDto, EpubReadDto, ImportBooksResult, ImportRejection},
};

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
    let book_title = normalize_book_title(&source_path);
    let cover_image_data = extract_cover_image_data_uri(&destination, format);

    if let Err(err) = repository::insert_imported_book(
      &conn,
      &book_title,
      format,
      &destination_text,
      cover_image_data.as_deref(),
    ) {
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

pub fn delete_book(app: AppHandle, book_id: i64) -> Result<(), String> {
  let conn = open_connection(&app)?;

  let file_paths = repository::list_book_file_paths(&conn, book_id)?;
  repository::delete_book_by_id(&conn, book_id)?;

  for path in file_paths {
    let file_path = PathBuf::from(path);

    if file_path.exists() {
      fs::remove_file(&file_path)
        .map_err(|err| format!("book removed from database but failed deleting local file: {err}"))?;
    }
  }

  Ok(())
}

pub fn read_epub(app: AppHandle, book_id: i64) -> Result<EpubReadDto, String> {
  let total_started_at = Instant::now();
  log::info!("[reader/open] backend read_epub started book_id={book_id}");

  let db_started_at = Instant::now();
  let conn = open_connection(&app)?;
  let (book_title, file_path, last_position, progress_percent) = repository::fetch_epub_read_context(&conn, book_id)?;
  log::info!(
    "[reader/open] backend book data fetched book_id={} elapsed_ms={}",
    book_id,
    db_started_at.elapsed().as_millis()
  );

  let exists_started_at = Instant::now();
  let epub_path = PathBuf::from(file_path);
  if !epub_path.exists() {
    return Err("arquivo EPUB não encontrado no disco local".to_string());
  }
  log::info!(
    "[reader/open] backend EPUB path checked book_id={} elapsed_ms={}",
    book_id,
    exists_started_at.elapsed().as_millis()
  );

  let parse_started_at = Instant::now();
  let chapters = read_epub_file(&epub_path)?;
  log::info!(
    "[reader/open] backend EPUB parsed book_id={} chapters={} elapsed_ms={}",
    book_id,
    chapters.len(),
    parse_started_at.elapsed().as_millis()
  );

  let progress_started_at = Instant::now();
  let last_chapter_index = parse_last_chapter_index(last_position, chapters.len());
  log::info!(
    "[reader/open] backend progress resolved book_id={} elapsed_ms={}",
    book_id,
    progress_started_at.elapsed().as_millis()
  );

  let result = EpubReadDto {
    book_id,
    book_title,
    chapters,
    last_chapter_index,
    progress_percent,
  };

  log::info!(
    "[reader/open] backend read_epub finished book_id={} total_ms={}",
    book_id,
    total_started_at.elapsed().as_millis()
  );

  Ok(result)
}

pub fn list_books(app: AppHandle) -> Result<Vec<BookDto>, String> {
  let conn = open_connection(&app)?;
  let missing_cover_sources = repository::list_books_missing_cover_sources(&conn)?;

  for (book_id, format, file_path) in missing_cover_sources {
    let path = PathBuf::from(&file_path);
    if !path.exists() {
      continue;
    }

    if let Some(cover_image_data) = extract_cover_image_data_uri(&path, &format) {
      repository::update_book_cover_image(&conn, book_id, &cover_image_data)?;
    }
  }

  repository::list_books(&conn)
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
