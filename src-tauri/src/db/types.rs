use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookDto {
  pub id: i64,
  pub title: String,
  pub author: String,
  pub description: String,
  pub cover_image_data: Option<String>,
  pub format: String,
  pub year: i64,
  pub progress: i64,
  pub tags: Vec<String>,
  pub is_epub_available: bool,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpubChapterDto {
  pub title: String,
  pub html: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpubReadDto {
  pub book_id: i64,
  pub book_title: String,
  pub chapters: Vec<EpubChapterDto>,
  pub last_chapter_index: i64,
  pub progress_percent: i64,
}
