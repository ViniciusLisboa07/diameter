use std::{
  ffi::OsStr,
  fs,
  io::Read,
  path::Path,
};

use regex::Regex;
use zip::ZipArchive;

use super::types::EpubChapterDto;

pub fn read_epub_file(epub_path: &Path) -> Result<Vec<EpubChapterDto>, String> {
  let epub_file = fs::File::open(epub_path).map_err(|err| format!("failed to open EPUB file: {err}"))?;
  let mut archive =
    ZipArchive::new(epub_file).map_err(|err| format!("failed to read EPUB archive: {err}"))?;

  let mut chapter_candidates: Vec<(String, String)> = Vec::new();

  for index in 0..archive.len() {
    let mut entry = archive
      .by_index(index)
      .map_err(|err| format!("failed to read EPUB entry: {err}"))?;

    if !entry.is_file() {
      continue;
    }

    let name = entry.name().to_string();
    if !(name.ends_with(".xhtml") || name.ends_with(".html") || name.ends_with(".htm")) {
      continue;
    }

    if name.contains("toc") || name.contains("nav") {
      continue;
    }

    let mut content = String::new();
    if entry.read_to_string(&mut content).is_err() || content.trim().is_empty() {
      continue;
    }

    let plain_text = sanitize_epub_html_to_text(&content)?;
    if plain_text.is_empty() {
      continue;
    }

    chapter_candidates.push((name, plain_text));
  }

  chapter_candidates.sort_by(|left, right| left.0.cmp(&right.0));

  let chapters = chapter_candidates
    .into_iter()
    .map(|(path, content)| EpubChapterDto {
      title: chapter_title_from_path(&path),
      content,
    })
    .collect::<Vec<_>>();

  if chapters.is_empty() {
    return Err("não foi possível extrair conteúdo textual deste EPUB".to_string());
  }

  Ok(chapters)
}

pub fn parse_last_chapter_index(last_position: Option<String>, chapter_count: usize) -> i64 {
  if chapter_count == 0 {
    return 0;
  }

  let Some(raw) = last_position else {
    return 0;
  };

  let parsed = raw
    .strip_prefix("chapter_index:")
    .and_then(|value| value.parse::<i64>().ok())
    .unwrap_or(0);

  let max_index = (chapter_count.saturating_sub(1)) as i64;
  parsed.clamp(0, max_index)
}

fn decode_basic_html_entities(input: &str) -> String {
  input
    .replace("&nbsp;", " ")
    .replace("&amp;", "&")
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&quot;", "\"")
    .replace("&#39;", "'")
}

fn sanitize_epub_html_to_text(input: &str) -> Result<String, String> {
  let script_style_re = Regex::new(r"(?is)<(script|style)[^>]*>.*?</(script|style)>")
    .map_err(|err| format!("failed to compile script/style regex: {err}"))?;
  let paragraph_break_re = Regex::new(r"(?i)</(p|div|section|article|h[1-6]|li|blockquote|tr)>")
    .map_err(|err| format!("failed to compile block regex: {err}"))?;
  let br_re = Regex::new(r"(?i)<br\s*/?>").map_err(|err| format!("failed to compile br regex: {err}"))?;
  let tag_re = Regex::new(r"(?is)<[^>]+>").map_err(|err| format!("failed to compile tag regex: {err}"))?;
  let whitespace_re = Regex::new(r"[ \t]+").map_err(|err| format!("failed to compile whitespace regex: {err}"))?;
  let line_break_re = Regex::new(r"\n{3,}").map_err(|err| format!("failed to compile line break regex: {err}"))?;

  let without_scripts = script_style_re.replace_all(input, " ");
  let with_breaks = paragraph_break_re.replace_all(&without_scripts, "\n");
  let with_line_breaks = br_re.replace_all(&with_breaks, "\n");
  let without_tags = tag_re.replace_all(&with_line_breaks, " ");
  let decoded = decode_basic_html_entities(&without_tags);
  let compact_spaces = whitespace_re.replace_all(&decoded, " ");
  let compact_breaks = line_break_re.replace_all(&compact_spaces, "\n\n");

  Ok(compact_breaks.trim().to_string())
}

fn chapter_title_from_path(path: &str) -> String {
  let file_name = Path::new(path)
    .file_stem()
    .and_then(OsStr::to_str)
    .unwrap_or("Capítulo");

  let normalized = file_name.replace(['_', '-'], " ").trim().to_string();
  if normalized.is_empty() {
    "Capítulo".to_string()
  } else {
    normalized
  }
}
