use std::{
    collections::HashMap,
    ffi::OsStr,
    fs,
    io::{Read, Seek},
    path::Path,
};

use regex::Regex;
use roxmltree::Document;
use zip::{read::ZipFile, ZipArchive};

use super::types::EpubChapterDto;

#[derive(Debug, Clone)]
struct ManifestItem {
    id: String,
    href: String,
    media_type: String,
    properties: String,
}

#[derive(Debug, Clone)]
struct EpubSection {
    path: String,
    html: String,
}

pub fn read_epub_file(epub_path: &Path) -> Result<Vec<EpubChapterDto>, String> {
    let epub_file =
        fs::File::open(epub_path).map_err(|err| format!("failed to open EPUB file: {err}"))?;
    let mut archive =
        ZipArchive::new(epub_file).map_err(|err| format!("failed to read EPUB archive: {err}"))?;

    let opf_path = find_opf_path(&mut archive)?;
    let (spine_paths, nav_paths, ncx_paths) = match opf_path.as_deref() {
        Some(path) => parse_package_document(&mut archive, path)?,
        None => (Vec::new(), Vec::new(), Vec::new()),
    };

    let mut title_by_path = HashMap::new();
    for path in nav_paths.iter().chain(ncx_paths.iter()) {
        if let Some(content) = read_zip_text_entry(&mut archive, path)? {
            let base_dir = parent_dir(path);
            let parsed_titles = if path.ends_with(".ncx") {
                parse_ncx_titles(&content, &base_dir)
            } else {
                parse_navigation_titles(&content, &base_dir)
            };

            for (target_path, title) in parsed_titles {
                title_by_path.entry(target_path).or_insert(title);
            }
        }
    }

    let sections = read_content_sections(&mut archive, &spine_paths)?;
    let chapters = sections
        .into_iter()
        .enumerate()
        .filter_map(|(index, section)| {
            build_chapter(section, index + 1, &title_by_path).transpose()
        })
        .collect::<Result<Vec<_>, String>>()?;

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

fn build_chapter(
    section: EpubSection,
    fallback_index: usize,
    title_by_path: &HashMap<String, String>,
) -> Result<Option<EpubChapterDto>, String> {
    let sanitized_html = sanitize_epub_html(&section.html)?;
    let content = sanitize_epub_html_to_text(&sanitized_html)?;
    if content.is_empty() {
        return Ok(None);
    }

    let toc_title = title_by_path
        .get(&section.path)
        .or_else(|| title_by_path.get(&strip_fragment(&section.path)))
        .filter(|title| is_useful_title(title));
    let html_title = extract_heading_title(&section.html).filter(|title| is_useful_title(title));
    let path_title = chapter_title_from_path(&section.path).filter(|title| is_useful_title(title));

    let title = toc_title
        .cloned()
        .or(html_title)
        .or(path_title)
        .unwrap_or_else(|| fallback_title_for_path(&section.path, fallback_index));

    Ok(Some(EpubChapterDto {
        title,
        content,
        html: sanitized_html,
    }))
}

fn find_opf_path<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<Option<String>, String> {
    if let Some(container_xml) = read_zip_text_entry(archive, "META-INF/container.xml")? {
        if let Ok(document) = Document::parse(&container_xml) {
            if let Some(path) = document.descendants().find_map(|node| {
                if node.is_element() && node.tag_name().name() == "rootfile" {
                    node.attribute("full-path").map(ToOwned::to_owned)
                } else {
                    None
                }
            }) {
                return Ok(Some(path));
            }
        }
    }

    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|err| format!("failed to inspect EPUB entry: {err}"))?;
        if entry.is_file() && entry.name().ends_with(".opf") {
            return Ok(Some(entry.name().to_string()));
        }
    }

    Ok(None)
}

fn parse_package_document<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    opf_path: &str,
) -> Result<(Vec<String>, Vec<String>, Vec<String>), String> {
    let Some(opf_xml) = read_zip_text_entry(archive, opf_path)? else {
        return Ok((Vec::new(), Vec::new(), Vec::new()));
    };
    let document = Document::parse(&opf_xml)
        .map_err(|err| format!("failed to parse EPUB package document: {err}"))?;
    let opf_dir = parent_dir(opf_path);

    let manifest = document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "item")
        .filter_map(|node| {
            let id = node.attribute("id")?.to_string();
            let href = normalize_epub_path(&opf_dir, node.attribute("href")?);
            let media_type = node.attribute("media-type").unwrap_or_default().to_string();
            let properties = node.attribute("properties").unwrap_or_default().to_string();
            Some(ManifestItem {
                id,
                href,
                media_type,
                properties,
            })
        })
        .collect::<Vec<_>>();
    let manifest_by_id = manifest
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect::<HashMap<_, _>>();

    let spine_node = document
        .descendants()
        .find(|node| node.is_element() && node.tag_name().name() == "spine");
    let spine_paths = spine_node
        .into_iter()
        .flat_map(|node| node.children())
        .filter(|node| node.is_element() && node.tag_name().name() == "itemref")
        .filter_map(|node| node.attribute("idref"))
        .filter_map(|idref| manifest_by_id.get(idref).copied())
        .filter(|item| is_html_media_type(&item.media_type) || is_html_path(&item.href))
        .map(|item| item.href.clone())
        .collect::<Vec<_>>();

    let nav_paths = manifest
        .iter()
        .filter(|item| {
            item.properties
                .split_whitespace()
                .any(|property| property == "nav")
        })
        .map(|item| item.href.clone())
        .collect::<Vec<_>>();
    let spine_toc_id = spine_node.and_then(|node| node.attribute("toc"));
    let mut ncx_paths = manifest
        .iter()
        .filter(|item| {
            Some(item.id.as_str()) == spine_toc_id
                || item.media_type == "application/x-dtbncx+xml"
                || item.href.ends_with(".ncx")
        })
        .map(|item| item.href.clone())
        .collect::<Vec<_>>();

    if ncx_paths.is_empty() {
        ncx_paths.extend(find_paths_by_suffix(archive, ".ncx")?);
    }

    Ok((spine_paths, nav_paths, ncx_paths))
}

fn read_content_sections<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    spine_paths: &[String],
) -> Result<Vec<EpubSection>, String> {
    let ordered_paths = if spine_paths.is_empty() {
        let mut paths = find_html_paths(archive)?;
        paths.retain(|path| !is_navigation_path(path));
        paths.sort();
        paths
    } else {
        dedupe_paths(spine_paths)
    };

    let mut sections = Vec::new();
    for path in ordered_paths {
        if let Some(html) = read_zip_text_entry(archive, &path)? {
            if !html.trim().is_empty() {
                sections.push(EpubSection { path, html });
            }
        }
    }

    Ok(sections)
}

fn parse_navigation_titles(input: &str, base_dir: &str) -> Vec<(String, String)> {
    let Ok(document) = Document::parse(input) else {
        return Vec::new();
    };

    document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "a")
        .filter_map(|node| {
            let href = node.attribute("href")?;
            let title = normalize_text(&node.text()?);
            if !is_useful_title(&title) {
                return None;
            }
            Some((strip_fragment(&normalize_epub_path(base_dir, href)), title))
        })
        .collect()
}

fn parse_ncx_titles(input: &str, base_dir: &str) -> Vec<(String, String)> {
    let Ok(document) = Document::parse(input) else {
        return Vec::new();
    };

    document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "navPoint")
        .filter_map(|nav_point| {
            let src = nav_point
                .descendants()
                .find(|node| node.is_element() && node.tag_name().name() == "content")?
                .attribute("src")?;
            let title = nav_point
                .descendants()
                .find(|node| node.is_element() && node.tag_name().name() == "text")?
                .text()
                .map(normalize_text)?;
            if !is_useful_title(&title) {
                return None;
            }
            Some((strip_fragment(&normalize_epub_path(base_dir, src)), title))
        })
        .collect()
}

fn extract_heading_title(input: &str) -> Option<String> {
    let document = Document::parse(input).ok()?;
    let title = document
        .descendants()
        .filter(|node| node.is_element())
        .find(|node| matches!(node.tag_name().name(), "h1" | "h2" | "h3" | "title"))?
        .text()
        .map(normalize_text)?;

    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

fn read_zip_text_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    path: &str,
) -> Result<Option<String>, String> {
    let Ok(mut entry) = archive.by_name(path) else {
        return Ok(None);
    };

    read_zip_file_to_string(&mut entry).map(Some)
}

fn read_zip_file_to_string(entry: &mut ZipFile<'_>) -> Result<String, String> {
    let mut content = String::new();
    entry
        .read_to_string(&mut content)
        .map_err(|err| format!("failed to read EPUB entry as text: {err}"))?;
    Ok(content)
}

fn find_html_paths<R: Read + Seek>(archive: &mut ZipArchive<R>) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|err| format!("failed to inspect EPUB entry: {err}"))?;
        if entry.is_file() && is_html_path(entry.name()) {
            paths.push(entry.name().to_string());
        }
    }
    Ok(paths)
}

fn find_paths_by_suffix<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    suffix: &str,
) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|err| format!("failed to inspect EPUB entry: {err}"))?;
        if entry.is_file() && entry.name().ends_with(suffix) {
            paths.push(entry.name().to_string());
        }
    }
    Ok(paths)
}

fn dedupe_paths(paths: &[String]) -> Vec<String> {
    let mut deduped = Vec::new();
    for path in paths {
        if !deduped.contains(path) {
            deduped.push(path.clone());
        }
    }
    deduped
}

fn is_html_media_type(media_type: &str) -> bool {
    matches!(media_type, "application/xhtml+xml" | "text/html")
}

fn is_html_path(path: &str) -> bool {
    path.ends_with(".xhtml") || path.ends_with(".html") || path.ends_with(".htm")
}

fn is_navigation_path(path: &str) -> bool {
    let lowered = path.to_lowercase();
    lowered.contains("toc") || lowered.contains("nav")
}

fn parent_dir(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .unwrap_or_default()
}

fn strip_fragment(path: &str) -> String {
    path.split('#').next().unwrap_or(path).to_string()
}

fn normalize_epub_path(base_dir: &str, href: &str) -> String {
    let href_without_fragment = strip_fragment(href);
    let combined = if base_dir.is_empty() || href_without_fragment.starts_with('/') {
        href_without_fragment.trim_start_matches('/').to_string()
    } else {
        format!("{base_dir}/{href_without_fragment}")
    };

    let mut segments = Vec::new();
    for segment in combined.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            _ => segments.push(segment),
        }
    }

    segments.join("/")
}

fn decode_basic_html_entities(input: &str) -> String {
    input
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn sanitize_epub_html(input: &str) -> Result<String, String> {
    let script_style_re = Regex::new(r"(?is)<(script|style|iframe|object|embed|svg|math)[^>]*>.*?</(script|style|iframe|object|embed|svg|math)>")
    .map_err(|err| format!("failed to compile unsafe block regex: {err}"))?;
    let comments_re = Regex::new(r"(?is)<!--.*?-->")
        .map_err(|err| format!("failed to compile comment regex: {err}"))?;
    let event_attr_re = Regex::new(r#"(?i)\s+on[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)"#)
        .map_err(|err| format!("failed to compile event attribute regex: {err}"))?;
    let javascript_url_re = Regex::new(
        r#"(?i)\s+(href|src)\s*=\s*("javascript:[^"]*"|'javascript:[^']*'|javascript:[^\s>]+)"#,
    )
    .map_err(|err| format!("failed to compile javascript URL regex: {err}"))?;
    let xml_declaration_re = Regex::new(r"(?is)<\?xml[^>]*>")
        .map_err(|err| format!("failed to compile XML regex: {err}"))?;
    let doctype_re = Regex::new(r"(?is)<!doctype[^>]*>")
        .map_err(|err| format!("failed to compile doctype regex: {err}"))?;
    let body_re = Regex::new(r"(?is)<body[^>]*>(.*?)</body>")
        .map_err(|err| format!("failed to compile body regex: {err}"))?;

    let without_scripts = script_style_re.replace_all(input, " ");
    let without_comments = comments_re.replace_all(&without_scripts, " ");
    let without_events = event_attr_re.replace_all(&without_comments, "");
    let without_javascript_urls = javascript_url_re.replace_all(&without_events, "");
    let without_xml = xml_declaration_re.replace_all(&without_javascript_urls, "");
    let without_doctype = doctype_re.replace_all(&without_xml, "");
    let body = body_re
        .captures(&without_doctype)
        .and_then(|captures| captures.get(1).map(|match_| match_.as_str().to_string()))
        .unwrap_or_else(|| without_doctype.to_string());

    Ok(body.trim().to_string())
}

fn sanitize_epub_html_to_text(input: &str) -> Result<String, String> {
    let paragraph_break_re = Regex::new(r"(?i)</(p|div|section|article|h[1-6]|li|blockquote|tr)>")
        .map_err(|err| format!("failed to compile block regex: {err}"))?;
    let br_re =
        Regex::new(r"(?i)<br\s*/?>").map_err(|err| format!("failed to compile br regex: {err}"))?;
    let tag_re =
        Regex::new(r"(?is)<[^>]+>").map_err(|err| format!("failed to compile tag regex: {err}"))?;
    let whitespace_re = Regex::new(r"[ \t]+")
        .map_err(|err| format!("failed to compile whitespace regex: {err}"))?;
    let line_break_re = Regex::new(r"\n{3,}")
        .map_err(|err| format!("failed to compile line break regex: {err}"))?;

    let with_breaks = paragraph_break_re.replace_all(input, "\n");
    let with_line_breaks = br_re.replace_all(&with_breaks, "\n");
    let without_tags = tag_re.replace_all(&with_line_breaks, " ");
    let decoded = decode_basic_html_entities(&without_tags);
    let compact_spaces = whitespace_re.replace_all(&decoded, " ");
    let compact_breaks = line_break_re.replace_all(&compact_spaces, "\n\n");

    Ok(compact_breaks.trim().to_string())
}

fn normalize_text(input: &str) -> String {
    let whitespace_re = Regex::new(r"\s+").expect("valid whitespace regex");
    decode_basic_html_entities(&whitespace_re.replace_all(input, " "))
        .trim()
        .to_string()
}

fn chapter_title_from_path(path: &str) -> Option<String> {
    let file_name = Path::new(path).file_stem().and_then(OsStr::to_str)?;
    let normalized = normalize_text(&file_name.replace(['_', '-'], " "));
    if normalized.is_empty() {
        None
    } else {
        Some(title_case_words(&normalized))
    }
}

fn fallback_title_for_path(path: &str, index: usize) -> String {
    let lowered = Path::new(path)
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_lowercase();

    if lowered.contains("section") || lowered.contains("secao") || lowered.contains("seção") {
        format!("Seção {index}")
    } else {
        format!("Capítulo {index}")
    }
}

fn is_useful_title(title: &str) -> bool {
    let normalized = normalize_text(title);
    !normalized.is_empty() && !is_technical_title(&normalized)
}

fn is_technical_title(title: &str) -> bool {
    let lowered = title.trim().to_lowercase().replace(['_', '-'], " ");
    let compact = lowered.replace(' ', "");
    let technical_re =
        Regex::new(r"^(part|section|chapter|chap|text|body|file|page|html|xhtml)0*\d+$")
            .expect("valid technical title regex");
    let numeric_re = Regex::new(r"^\d+$").expect("valid numeric title regex");
    technical_re.is_match(&compact) || numeric_re.is_match(&compact)
}

fn title_case_words(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
