use std::{
  fs,
  io::Read,
  path::{Path, PathBuf},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use roxmltree::Document;
use zip::ZipArchive;

const MAX_COVER_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const MIN_PDF_IMAGE_BYTES: usize = 1024;

type ImageRange = (usize, usize, &'static str);

#[derive(Clone)]
struct EpubManifestItem {
  id: String,
  href: String,
  media_type: Option<String>,
  properties: Option<String>,
}

pub fn extract_cover_image_data_uri(file_path: &Path, format: &str) -> Option<String> {
  match format {
    "EPUB" => extract_epub_cover_data_uri(file_path),
    "PDF" => extract_pdf_cover_data_uri(file_path),
    _ => None,
  }
}

fn extract_epub_cover_data_uri(epub_path: &Path) -> Option<String> {
  let epub_file = fs::File::open(epub_path).ok()?;
  let mut archive = ZipArchive::new(epub_file).ok()?;

  let container_xml = read_zip_text(&mut archive, "META-INF/container.xml")?;
  let opf_path = find_opf_path(&container_xml)?;
  let opf_xml = read_zip_text(&mut archive, &opf_path)?;

  let (cover_href, media_type) = find_epub_cover_entry(&opf_xml)?;
  let cover_path = resolve_relative_archive_path(&opf_path, &cover_href);

  let bytes = read_zip_binary(&mut archive, &cover_path)?;
  let mime_type = media_type
    .as_deref()
    .filter(|mime| mime.starts_with("image/"))
    .or_else(|| mime_from_path(&cover_path))
    .or_else(|| sniff_image_mime_type(&bytes))?;

  encode_data_uri(bytes, mime_type)
}

fn extract_pdf_cover_data_uri(pdf_path: &Path) -> Option<String> {
  let bytes = fs::read(pdf_path).ok()?;
  if bytes.is_empty() {
    return None;
  }

  let mut candidates = find_embedded_jpeg_ranges(&bytes);
  candidates.extend(find_embedded_png_ranges(&bytes));

  let best = candidates
    .into_iter()
    .max_by_key(|(start, end, _)| end.saturating_sub(*start))?;

  let (start, end, mime_type) = best;
  if end <= start || end > bytes.len() {
    return None;
  }

  encode_data_uri(bytes[start..end].to_vec(), mime_type)
}

fn find_embedded_jpeg_ranges(bytes: &[u8]) -> Vec<ImageRange> {
  let mut ranges = Vec::new();
  let mut index = 0_usize;

  while index + 3 < bytes.len() {
    if bytes[index] == 0xFF && bytes[index + 1] == 0xD8 && bytes[index + 2] == 0xFF {
      let start = index;
      index += 3;

      while index + 1 < bytes.len() {
        if bytes[index] == 0xFF && bytes[index + 1] == 0xD9 {
          let end = index + 2;
          if end.saturating_sub(start) >= MIN_PDF_IMAGE_BYTES {
            ranges.push((start, end, "image/jpeg"));
          }
          index = end;
          break;
        }
        index += 1;
      }
    } else {
      index += 1;
    }
  }

  ranges
}

fn find_embedded_png_ranges(bytes: &[u8]) -> Vec<ImageRange> {
  let mut ranges = Vec::new();
  let mut index = 0_usize;

  let signature: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
  let iend_marker: [u8; 8] = [b'I', b'E', b'N', b'D', 0xAE, b'B', 0x60, 0x82];

  while index + signature.len() <= bytes.len() {
    if bytes[index..].starts_with(&signature) {
            if let Some(relative_end) =
                find_subsequence(&bytes[index + signature.len()..], &iend_marker)
            {
        let end = index + signature.len() + relative_end + iend_marker.len();
        if end.saturating_sub(index) >= MIN_PDF_IMAGE_BYTES {
          ranges.push((index, end, "image/png"));
        }
        index = end;
        continue;
      }
    }

    index += 1;
  }

  ranges
}

fn find_subsequence(buffer: &[u8], needle: &[u8]) -> Option<usize> {
  if needle.is_empty() || needle.len() > buffer.len() {
    return None;
  }

    buffer
        .windows(needle.len())
        .position(|window| window == needle)
}

fn find_opf_path(container_xml: &str) -> Option<String> {
  let document = Document::parse(container_xml).ok()?;

  document
    .descendants()
    .find(|node| node.tag_name().name() == "rootfile")
    .and_then(|node| node.attribute("full-path"))
    .map(normalize_archive_path)
    .filter(|path| !path.is_empty())
}

fn find_epub_cover_entry(opf_xml: &str) -> Option<(String, Option<String>)> {
  let document = Document::parse(opf_xml).ok()?;

  let manifest_items = document
    .descendants()
    .filter(|node| node.tag_name().name() == "item")
    .filter_map(|node| {
      let id = node.attribute("id")?.trim().to_string();
      let href = node.attribute("href")?.trim().to_string();

      if id.is_empty() || href.is_empty() {
        return None;
      }

      Some(EpubManifestItem {
        id,
        href,
        media_type: node.attribute("media-type").map(str::to_string),
        properties: node.attribute("properties").map(str::to_string),
      })
    })
    .collect::<Vec<_>>();

  let cover_id = document
    .descendants()
    .find(|node| {
      node.tag_name().name() == "meta"
        && node
          .attribute("name")
          .map(|name| name.eq_ignore_ascii_case("cover"))
          .unwrap_or(false)
    })
    .and_then(|node| node.attribute("content"))
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(str::to_string);

  if let Some(id) = cover_id {
    if let Some(item) = manifest_items.iter().find(|item| item.id == id) {
      return Some((item.href.clone(), item.media_type.clone()));
    }
  }

  if let Some(item) = manifest_items.iter().find(|item| {
        item.properties
      .as_deref()
            .map(|properties| {
                properties
                    .split_whitespace()
                    .any(|token| token == "cover-image")
            })
      .unwrap_or(false)
  }) {
    return Some((item.href.clone(), item.media_type.clone()));
  }

  if let Some(reference_href) = document
    .descendants()
    .find(|node| {
      node.tag_name().name() == "reference"
        && node
          .attribute("type")
          .map(|value| value.eq_ignore_ascii_case("cover"))
          .unwrap_or(false)
    })
    .and_then(|node| node.attribute("href"))
    .map(str::trim)
    .filter(|href| !href.is_empty())
  {
    return Some((reference_href.to_string(), None));
  }

  manifest_items
    .iter()
    .find(|item| {
            item.media_type
        .as_deref()
        .map(|mime| mime.starts_with("image/"))
        .unwrap_or_else(|| mime_from_path(&item.href).is_some())
        && item.href.to_lowercase().contains("cover")
    })
    .map(|item| (item.href.clone(), item.media_type.clone()))
}

fn resolve_relative_archive_path(opf_path: &str, href: &str) -> String {
  let clean_href = normalize_archive_path(href);
  if clean_href.is_empty() {
    return clean_href;
  }

  let base_dir = Path::new(opf_path)
    .parent()
    .map(Path::to_path_buf)
    .unwrap_or_else(PathBuf::new);

  let joined = base_dir.join(clean_href);
  normalize_archive_path(joined.to_string_lossy().as_ref())
}

fn normalize_archive_path(raw_path: &str) -> String {
  let mut parts = Vec::new();

  for part in raw_path.split(['/', '\\']) {
    if part.is_empty() || part == "." {
      continue;
    }

    if part == ".." {
      let _ = parts.pop();
      continue;
    }

    parts.push(part);
  }

  parts.join("/")
}

fn read_zip_text<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    path: &str,
) -> Option<String> {
  let normalized_path = normalize_archive_path(path);
  if normalized_path.is_empty() {
    return None;
  }

  let mut entry = archive.by_name(&normalized_path).ok()?;
  let mut content = String::new();
  entry.read_to_string(&mut content).ok()?;
  Some(content)
}

fn read_zip_binary<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    path: &str,
) -> Option<Vec<u8>> {
  let normalized_path = normalize_archive_path(path);
  if normalized_path.is_empty() {
    return None;
  }

  let mut entry = archive.by_name(&normalized_path).ok()?;
  let mut content = Vec::new();
  entry.read_to_end(&mut content).ok()?;
  Some(content)
}

fn encode_data_uri(bytes: Vec<u8>, mime_type: &str) -> Option<String> {
  if bytes.is_empty() || bytes.len() > MAX_COVER_IMAGE_BYTES {
    return None;
  }

    Some(format!(
        "data:{mime_type};base64,{}",
        STANDARD.encode(bytes)
    ))
}

fn mime_from_path(path: &str) -> Option<&'static str> {
  let extension = Path::new(path)
    .extension()
    .and_then(|value| value.to_str())
    .map(|value| value.to_lowercase())?;

  match extension.as_str() {
    "jpg" | "jpeg" => Some("image/jpeg"),
    "png" => Some("image/png"),
    "gif" => Some("image/gif"),
    "webp" => Some("image/webp"),
    "bmp" => Some("image/bmp"),
    "svg" => Some("image/svg+xml"),
    _ => None,
  }
}

fn sniff_image_mime_type(bytes: &[u8]) -> Option<&'static str> {
  if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
    return Some("image/jpeg");
  }

  if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
    return Some("image/png");
  }

  if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
    return Some("image/gif");
  }

  if bytes.starts_with(b"RIFF") && bytes.len() > 12 && bytes[8..12] == *b"WEBP" {
    return Some("image/webp");
  }

  if bytes.starts_with(&[0x42, 0x4D]) {
    return Some("image/bmp");
  }

  None
}
