//! Document import (PDF/DOCX text extraction) and rendering of generated
//! resumes and cover letters to HTML, DOCX and PDF.

pub mod import;
pub mod render;

pub use import::{extract_text, import_master_resume};
pub use render::{
    render_cover_letter_bundle, render_resume_bundle, resume_to_html, resume_to_plain_text,
    RenderedFiles,
};
