use std::{
  ffi::OsStr,
  fs,
  path::{Path, PathBuf},
};

use tauri::AppHandle;

use super::{
  connection::{library_dir, open_connection},
  epub::{parse_last_chapter_index, read_epub_file},
  repository,
  types::{EpubReadDto, ImportBooksResult, ImportRejection},
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

    if let Err(err) = repository::insert_imported_book(&conn, &book_title, format, &destination_text) {
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

pub fn read_epub(app: AppHandle, book_id: i64) -> Result<EpubReadDto, String> {
  let conn = open_connection(&app)?;
  let (book_title, file_path, last_position, progress_percent) = repository::fetch_epub_read_context(&conn, book_id)?;

  let epub_path = PathBuf::from(file_path);
  if !epub_path.exists() {
    return Err("arquivo EPUB não encontrado no disco local".to_string());
  }

  let chapters = read_epub_file(&epub_path)?;
  let last_chapter_index = parse_last_chapter_index(last_position, chapters.len());

  Ok(EpubReadDto {
    book_id,
    book_title,
    chapters,
    last_chapter_index,
    progress_percent,
  })
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
