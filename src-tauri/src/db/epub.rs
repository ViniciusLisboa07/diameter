use std::{
    collections::HashMap,
    ffi::OsStr,
    fs,
    io::{Read, Seek},
    path::Path,
    time::Instant,
};

use base64::{engine::general_purpose, Engine as _};
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

#[derive(Debug, Clone)]
struct BuiltChapter {
    title: Option<String>,
    fallback_title: String,
    html: String,
}

pub fn read_epub_file(epub_path: &Path) -> Result<Vec<EpubChapterDto>, String> {
    let total_started_at = Instant::now();
    log::info!(
        "[reader/open] EPUB read started path={}",
        epub_path.display()
    );

    let open_started_at = Instant::now();
    let epub_file =
        fs::File::open(epub_path).map_err(|err| format!("failed to open EPUB file: {err}"))?;
    log::info!(
        "[reader/open] EPUB file opened elapsed_ms={}",
        open_started_at.elapsed().as_millis()
    );

    let zip_started_at = Instant::now();
    let mut archive =
        ZipArchive::new(epub_file).map_err(|err| format!("failed to read EPUB archive: {err}"))?;
    log::info!(
        "[reader/open] EPUB ZIP archive initialized entries={} elapsed_ms={}",
        archive.len(),
        zip_started_at.elapsed().as_millis()
    );

    let opf_started_at = Instant::now();
    let opf_path = find_opf_path(&mut archive)?;
    log::info!(
        "[reader/open] EPUB OPF path resolved found={} elapsed_ms={}",
        opf_path.is_some(),
        opf_started_at.elapsed().as_millis()
    );

    let package_started_at = Instant::now();
    let (spine_paths, nav_paths, ncx_paths) = match opf_path.as_deref() {
        Some(path) => parse_package_document(&mut archive, path)?,
        None => (Vec::new(), Vec::new(), Vec::new()),
    };
    log::info!(
        "[reader/open] EPUB package parsed spine_paths={} nav_paths={} ncx_paths={} elapsed_ms={}",
        spine_paths.len(),
        nav_paths.len(),
        ncx_paths.len(),
        package_started_at.elapsed().as_millis()
    );

    let nav_started_at = Instant::now();
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
    log::info!(
        "[reader/open] EPUB navigation titles parsed titles={} elapsed_ms={}",
        title_by_path.len(),
        nav_started_at.elapsed().as_millis()
    );

    let sections_started_at = Instant::now();
    let sections = read_content_sections(&mut archive, &spine_paths)?;
    log::info!(
        "[reader/open] EPUB content sections read sections={} elapsed_ms={}",
        sections.len(),
        sections_started_at.elapsed().as_millis()
    );

    let chapters_started_at = Instant::now();
    let built_chapters = sections
        .into_iter()
        .enumerate()
        .filter_map(|(index, section)| {
            build_chapter(&mut archive, section, index + 1, &title_by_path).transpose()
        })
        .collect::<Result<Vec<_>, String>>()?;
    let chapters = merge_untitled_chapters(built_chapters);
    log::info!(
        "[reader/open] EPUB chapters built chapters={} elapsed_ms={}",
        chapters.len(),
        chapters_started_at.elapsed().as_millis()
    );

    if chapters.is_empty() {
        return Err("não foi possível extrair conteúdo textual deste EPUB".to_string());
    }

    log::info!(
        "[reader/open] EPUB read finished chapters={} total_ms={}",
        chapters.len(),
        total_started_at.elapsed().as_millis()
    );

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

fn build_chapter<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    section: EpubSection,
    fallback_index: usize,
    title_by_path: &HashMap<String, String>,
) -> Result<Option<BuiltChapter>, String> {
    let sanitized_html = sanitize_epub_html(&section.html)?;
    let html = embed_epub_images(archive, &section.path, &sanitized_html)?;
    let content = sanitize_epub_html_to_text(&html)?;
    if content.is_empty() && !has_image_tag(&html) {
        return Ok(None);
    }

    let toc_title = title_by_path
        .get(&section.path)
        .or_else(|| title_by_path.get(&strip_fragment(&section.path)))
        .filter(|title| is_useful_title(title));
    let html_title = extract_heading_title(&section.html).filter(|title| is_useful_title(title));
    let path_title = chapter_title_from_path(&section.path).filter(|title| is_useful_title(title));

    let title = toc_title.cloned().or(html_title).or(path_title);

    Ok(Some(BuiltChapter {
        title,
        fallback_title: fallback_title_for_path(&section.path, fallback_index),
        html,
    }))
}

fn merge_untitled_chapters(chapters: Vec<BuiltChapter>) -> Vec<EpubChapterDto> {
    if chapters.iter().all(|chapter| chapter.title.is_none()) {
        return chapters
            .into_iter()
            .map(|chapter| EpubChapterDto {
                title: chapter.fallback_title,
                html: chapter.html,
            })
            .collect();
    }

    let mut merged = Vec::<EpubChapterDto>::new();
    let mut leading_fragments = Vec::<String>::new();

    for chapter in chapters {
        match chapter.title {
            Some(title) => {
                let html = if leading_fragments.is_empty() {
                    chapter.html
                } else {
                    let mut fragments = std::mem::take(&mut leading_fragments);
                    fragments.push(chapter.html);
                    join_html_fragments(fragments)
                };
                merged.push(EpubChapterDto { title, html });
            }
            None => {
                if let Some(previous) = merged.last_mut() {
                    previous.html =
                        join_html_fragments(vec![std::mem::take(&mut previous.html), chapter.html]);
                } else {
                    leading_fragments.push(chapter.html);
                }
            }
        }
    }

    if !leading_fragments.is_empty() {
        let title = format!("Capítulo {}", merged.len() + 1);
        merged.push(EpubChapterDto {
            title,
            html: join_html_fragments(leading_fragments),
        });
    }

    merged
}

fn join_html_fragments(fragments: Vec<String>) -> String {
    fragments
        .into_iter()
        .filter(|fragment| !fragment.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
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

    let heading_title = document
        .descendants()
        .filter(|node| node.is_element())
        .find(|node| matches!(node.tag_name().name(), "h1" | "h2" | "h3"))
        .map(extract_node_text)
        .filter(|title| !title.is_empty());
    if heading_title.is_some() {
        return heading_title;
    }

    document
        .descendants()
        .filter(|node| node.is_element())
        .find(|node| node.tag_name().name() == "title")
        .map(extract_node_text)
        .filter(|title| !title.is_empty())
}

fn extract_node_text(node: roxmltree::Node<'_, '_>) -> String {
    normalize_text(
        &node
            .descendants()
            .filter(|descendant| descendant.is_text())
            .filter_map(|descendant| descendant.text())
            .collect::<Vec<_>>()
            .join(" "),
    )
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

fn read_zip_binary_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    path: &str,
) -> Result<Option<Vec<u8>>, String> {
    let Ok(mut entry) = archive.by_name(path) else {
        return Ok(None);
    };

    let mut content = Vec::new();
    entry
        .read_to_end(&mut content)
        .map_err(|err| format!("failed to read EPUB entry as binary: {err}"))?;
    Ok(Some(content))
}

fn embed_epub_images<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    section_path: &str,
    input: &str,
) -> Result<String, String> {
    let src_attr_re = Regex::new(r#"(?i)\s+(src)\s*=\s*("([^"]*)"|'([^']*)'|([^\s>]+))"#)
        .map_err(|err| format!("failed to compile image source regex: {err}"))?;
    let base_dir = parent_dir(section_path);
    let mut output = String::with_capacity(input.len());
    let mut last_end = 0;

    for captures in src_attr_re.captures_iter(input) {
        let Some(full_match) = captures.get(0) else {
            continue;
        };
        output.push_str(&input[last_end..full_match.start()]);

        let raw_src = captures
            .get(3)
            .or_else(|| captures.get(4))
            .or_else(|| captures.get(5))
            .map(|match_| match_.as_str())
            .unwrap_or_default();

        if let Some(data_uri) = epub_image_data_uri(archive, &base_dir, raw_src) {
            output.push_str(&format!(" src=\"{data_uri}\""));
        } else {
            output.push_str(full_match.as_str());
        }

        last_end = full_match.end();
    }

    output.push_str(&input[last_end..]);
    Ok(output)
}

fn epub_image_data_uri<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    base_dir: &str,
    src: &str,
) -> Option<String> {
    if !is_local_epub_resource(src) {
        return None;
    }

    let decoded_src = percent_decode(&decode_basic_html_entities(src.trim()));
    let src_without_suffix = decoded_src.split(['#', '?']).next().unwrap_or(&decoded_src);
    let image_path = normalize_epub_path(base_dir, src_without_suffix);
    let media_type = image_media_type(&image_path)?;
    let image_bytes = read_zip_binary_entry(archive, &image_path).ok().flatten()?;
    if image_bytes.is_empty() {
        return None;
    }

    Some(format!(
        "data:{media_type};base64,{}",
        general_purpose::STANDARD.encode(image_bytes)
    ))
}

fn is_local_epub_resource(src: &str) -> bool {
    let lowered = src.trim().to_lowercase();
    !lowered.is_empty()
        && !lowered.starts_with('#')
        && !lowered.starts_with("data:")
        && !lowered.starts_with("http:")
        && !lowered.starts_with("https:")
        && !lowered.starts_with("mailto:")
}

fn image_media_type(path: &str) -> Option<&'static str> {
    let extension = Path::new(path)
        .extension()
        .and_then(OsStr::to_str)?
        .to_lowercase();

    match extension.as_str() {
        "avif" => Some("image/avif"),
        "bmp" => Some("image/bmp"),
        "gif" => Some("image/gif"),
        "jpe" | "jpeg" | "jpg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                decoded.push(high * 16 + low);
                index += 3;
                continue;
            }
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&decoded).to_string()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
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

fn has_image_tag(input: &str) -> bool {
    let image_re = Regex::new(r"(?i)<img\b").expect("valid image tag regex");
    image_re.is_match(input)
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
    let normalized = normalize_text(title);
    let raw_lowered = normalized.trim().to_lowercase();
    let lowered = raw_lowered.replace(['_', '-'], " ");
    let compact = lowered.replace(' ', "");
    let without_fragment = raw_lowered.split('#').next().unwrap_or(&raw_lowered);
    let extension = Path::new(without_fragment)
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    let generic_titles = [
        "unknown",
        "untitled",
        "sem titulo",
        "sem título",
        "titulo desconhecido",
        "título desconhecido",
    ];
    let technical_re =
        Regex::new(r"^(part|section|chapter|chap|text|body|file|page|html|xhtml)0*\d+$")
            .expect("valid technical title regex");
    let short_alpha_numeric_re =
        Regex::new(r"^[a-z]{1,3}0*\d+[a-z]?$").expect("valid short alpha numeric title regex");
    let numeric_re = Regex::new(r"^\d+$").expect("valid numeric title regex");
    matches!(extension, "xhtml" | "html" | "htm")
        || generic_titles.contains(&lowered.as_str())
        || technical_re.is_match(&compact)
        || short_alpha_numeric_re.is_match(&compact)
        || numeric_re.is_match(&compact)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::{write::SimpleFileOptions, ZipWriter};

    #[test]
    fn rejects_epub_file_names_as_titles() {
        assert!(!is_useful_title("a2.xhtml"));
        assert!(!is_useful_title("Text/a2.xhtml"));
        assert!(!is_useful_title("A2"));
        assert!(!is_useful_title("Unknown"));
        assert!(is_useful_title("A coragem de ser imperfeito"));
    }

    #[test]
    fn heading_title_is_preferred_over_technical_document_title() {
        let html = r#"
            <html>
                <head><title>a2.xhtml</title></head>
                <body><h1>Capitulo real</h1><p>Conteudo.</p></body>
            </html>
        "#;

        assert_eq!(
            extract_heading_title(html).as_deref(),
            Some("Capitulo real")
        );
    }

    #[test]
    fn untitled_fragments_are_merged_into_previous_clear_chapter() {
        let chapters = merge_untitled_chapters(vec![
            BuiltChapter {
                title: Some("Capitulo real".to_string()),
                fallback_title: "Capítulo 1".to_string(),
                html: "<h1>Capitulo real</h1><p>Parte principal.</p>".to_string(),
            },
            BuiltChapter {
                title: None,
                fallback_title: "Capítulo 2".to_string(),
                html: "<p>Fragmento sem titulo claro.</p>".to_string(),
            },
        ]);

        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].title, "Capitulo real");
        assert!(chapters[0].html.contains("Parte principal."));
        assert!(chapters[0].html.contains("Fragmento sem titulo claro."));
    }

    #[test]
    fn local_epub_images_are_embedded_as_data_uris() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        writer
            .start_file("OEBPS/images/photo.jpg", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(&[0xff, 0xd8, 0xff]).unwrap();
        let cursor = writer.finish().unwrap();
        let mut archive = ZipArchive::new(cursor).unwrap();

        let html = r#"<p><img src="../images/photo.jpg" alt="Foto"></p>"#;
        let embedded = embed_epub_images(&mut archive, "OEBPS/Text/chapter.xhtml", html).unwrap();

        assert!(embedded.contains(r#"src="data:image/jpeg;base64,/9j/""#));
        assert!(embedded.contains(r#"alt="Foto""#));
    }
}
