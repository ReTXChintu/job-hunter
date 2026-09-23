use std::io::Read;
use std::path::{Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::domain::{MasterResumeRef, ResumeFormat};
use crate::error::{CoreError, CoreResult};
use crate::util::{new_id, now};

/// Extract plain text from a PDF, DOCX, TXT or MD file.
pub fn extract_text(path: &Path) -> CoreResult<String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "pdf" => extract_pdf(path),
        "docx" => extract_docx(path),
        "txt" | "md" => Ok(std::fs::read_to_string(path)?),
        other => Err(CoreError::DocumentImport(format!(
            "unsupported file type '.{other}'; use PDF or DOCX"
        ))),
    }
}

fn extract_pdf(path: &Path) -> CoreResult<String> {
    let bytes = std::fs::read(path)?;
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| CoreError::DocumentImport(format!("could not read PDF text: {e}")))?;
    let cleaned = clean_text(&text);
    if cleaned.trim().is_empty() {
        return Err(CoreError::DocumentImport(
            "the PDF contains no extractable text (it may be a scanned image)".into(),
        ));
    }
    Ok(cleaned)
}

fn extract_docx(path: &Path) -> CoreResult<String> {
    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| CoreError::DocumentImport(format!("not a valid DOCX: {e}")))?;
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .map_err(|_| CoreError::DocumentImport("DOCX has no word/document.xml".into()))?
        .read_to_string(&mut xml)?;
    Ok(docx_xml_to_text(&xml))
}

/// Walk WordprocessingML and emit paragraphs separated by newlines.
pub fn docx_xml_to_text(xml: &str) -> String {
    let mut reader = Reader::from_str(xml);
    let mut out = String::new();
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => match e.local_name().as_ref() {
                b"tab" => out.push('\t'),
                b"br" | b"cr" => out.push('\n'),
                _ => {}
            },
            Ok(Event::End(e)) => {
                if e.local_name().as_ref() == b"p" {
                    out.push('\n');
                }
            }
            Ok(Event::Text(t)) => {
                if let Ok(text) = t.decode() {
                    out.push_str(&text);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    clean_text(&out)
}

fn clean_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank_run = 0;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            blank_run += 1;
            if blank_run <= 1 {
                out.push('\n');
            }
        } else {
            blank_run = 0;
            out.push_str(trimmed);
            out.push('\n');
        }
    }
    out.trim().to_string()
}

/// Copy the original resume into the app data directory (never modified),
/// extract its text and return the reference to store on the profile.
pub fn import_master_resume(source: &Path, master_dir: &Path) -> CoreResult<MasterResumeRef> {
    if !source.is_file() {
        return Err(CoreError::DocumentImport(format!(
            "file not found: {}",
            source.display()
        )));
    }
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    let format = match ext.as_str() {
        "pdf" => ResumeFormat::Pdf,
        "docx" => ResumeFormat::Docx,
        _ => {
            return Err(CoreError::DocumentImport(
                "only PDF and DOCX resumes can be imported".into(),
            ))
        }
    };
    let text = extract_text(source)?;
    std::fs::create_dir_all(master_dir)?;
    let id = new_id();
    let original_name = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "resume".into());
    let stamp = now().format("%Y%m%d-%H%M%S");
    let stored: PathBuf = master_dir.join(format!("master-{stamp}.{ext}"));
    std::fs::copy(source, &stored)?;
    let text_path = master_dir.join(format!("master-{stamp}.txt"));
    std::fs::write(&text_path, &text)?;
    Ok(MasterResumeRef {
        id,
        original_file_name: original_name,
        stored_path: stored.to_string_lossy().to_string(),
        text_path: text_path.to_string_lossy().to_string(),
        format,
        imported_at: now(),
        text_chars: text.chars().count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docx_xml_paragraphs_become_lines() {
        let xml = r#"<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Asha</w:t><w:tab/><w:t>Verma</w:t></w:r></w:p><w:p><w:r><w:t>Full Stack</w:t></w:r></w:p></w:body></w:document>"#;
        assert_eq!(docx_xml_to_text(xml), "Asha\tVerma\nFull Stack");
    }

    #[test]
    fn unsupported_extension_is_rejected() {
        let err = extract_text(Path::new("resume.pages")).unwrap_err();
        assert!(matches!(err, CoreError::DocumentImport(_)));
    }
}
