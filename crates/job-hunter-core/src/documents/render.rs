use std::path::{Path, PathBuf};

use docx_rs::*;
use html_escape::encode_text as esc;
use serde::{Deserialize, Serialize};

use crate::domain::ResumeDocument;
use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedFiles {
    pub docx: Option<String>,
    pub pdf: Option<String>,
    pub html: Option<String>,
    pub json: Option<String>,
    pub txt: Option<String>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// HTML (ATS friendly: single column, standard headings, no tables/graphics)
// ---------------------------------------------------------------------------

const CSS: &str = r#"
body { font-family: Arial, Helvetica, sans-serif; font-size: 10.5pt; color: #111; margin: 0; padding: 0; line-height: 1.35; }
.page { max-width: 7.3in; margin: 0 auto; padding: 0.55in 0.6in; }
h1 { font-size: 20pt; margin: 0 0 2pt 0; letter-spacing: 0.2px; }
.headline { font-size: 11.5pt; font-weight: bold; margin: 0 0 4pt 0; }
.contact { font-size: 9.5pt; color: #333; margin-bottom: 10pt; }
.contact span + span::before { content: " | "; color: #888; }
h2 { font-size: 11pt; text-transform: uppercase; letter-spacing: 0.6px; border-bottom: 1px solid #999; padding-bottom: 2pt; margin: 12pt 0 6pt 0; }
p { margin: 0 0 5pt 0; }
ul { margin: 2pt 0 6pt 0; padding-left: 16pt; }
li { margin: 0 0 2pt 0; }
.entry { margin-bottom: 7pt; }
.entry-head { font-weight: bold; }
.entry-sub { color: #333; font-size: 9.8pt; }
.skills p { margin: 0 0 2pt 0; }
@page { size: A4; margin: 0; }
@media print { .page { padding: 0.5in 0.55in; } }
"#;

fn join_dates(start: &str, end: &str) -> String {
    match (start.trim(), end.trim()) {
        ("", "") => String::new(),
        (s, "") => format!("{s} – Present"),
        ("", e) => e.to_string(),
        (s, e) => format!("{s} – {e}"),
    }
}

pub fn resume_to_html(doc: &ResumeDocument) -> String {
    let mut h = String::new();
    h.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>");
    h.push_str(&esc(&doc.contact.name));
    h.push_str(" - Resume</title><style>");
    h.push_str(CSS);
    h.push_str("</style></head><body><div class=\"page\">");
    h.push_str(&format!("<h1>{}</h1>", esc(&doc.contact.name)));
    if !doc.headline.trim().is_empty() {
        h.push_str(&format!(
            "<div class=\"headline\">{}</div>",
            esc(&doc.headline)
        ));
    }
    let mut contact: Vec<String> = Vec::new();
    for v in [
        &doc.contact.location,
        &doc.contact.email,
        &doc.contact.phone,
        &doc.contact.linkedin,
        &doc.contact.github,
        &doc.contact.portfolio,
    ] {
        if !v.trim().is_empty() {
            contact.push(format!("<span>{}</span>", esc(v)));
        }
    }
    if !contact.is_empty() {
        h.push_str(&format!(
            "<div class=\"contact\">{}</div>",
            contact.join("")
        ));
    }
    if !doc.summary.trim().is_empty() {
        h.push_str("<h2>Summary</h2>");
        h.push_str(&format!("<p>{}</p>", esc(&doc.summary)));
    }
    let skills: Vec<&crate::domain::ResumeSkillSection> =
        doc.skills.iter().filter(|s| !s.items.is_empty()).collect();
    if !skills.is_empty() {
        h.push_str("<h2>Skills</h2><div class=\"skills\">");
        for s in skills {
            h.push_str(&format!(
                "<p><strong>{}:</strong> {}</p>",
                esc(&s.name),
                esc(&s.items.join(", "))
            ));
        }
        h.push_str("</div>");
    }
    if !doc.experience.is_empty() {
        h.push_str("<h2>Experience</h2>");
        for e in &doc.experience {
            h.push_str("<div class=\"entry\">");
            h.push_str(&format!(
                "<div class=\"entry-head\">{} — {}</div>",
                esc(&e.role),
                esc(&e.company)
            ));
            let mut sub = Vec::new();
            let dates = join_dates(&e.start_date, &e.end_date);
            if !dates.is_empty() {
                sub.push(esc(&dates).to_string());
            }
            if !e.location.trim().is_empty() {
                sub.push(esc(&e.location).to_string());
            }
            if !sub.is_empty() {
                h.push_str(&format!(
                    "<div class=\"entry-sub\">{}</div>",
                    sub.join(" | ")
                ));
            }
            if !e.bullets.is_empty() {
                h.push_str("<ul>");
                for b in &e.bullets {
                    h.push_str(&format!("<li>{}</li>", esc(b)));
                }
                h.push_str("</ul>");
            }
            h.push_str("</div>");
        }
    }
    if !doc.projects.is_empty() {
        h.push_str("<h2>Projects</h2>");
        for p in &doc.projects {
            h.push_str("<div class=\"entry\">");
            let mut head = esc(&p.name).to_string();
            if !p.technologies.is_empty() {
                head.push_str(&format!(
                    " <span class=\"entry-sub\">({})</span>",
                    esc(&p.technologies.join(", "))
                ));
            }
            h.push_str(&format!("<div class=\"entry-head\">{head}</div>"));
            if !p.description.trim().is_empty() {
                h.push_str(&format!("<p>{}</p>", esc(&p.description)));
            }
            if !p.bullets.is_empty() {
                h.push_str("<ul>");
                for b in &p.bullets {
                    h.push_str(&format!("<li>{}</li>", esc(b)));
                }
                h.push_str("</ul>");
            }
            if !p.url.trim().is_empty() {
                h.push_str(&format!("<p class=\"entry-sub\">{}</p>", esc(&p.url)));
            }
            h.push_str("</div>");
        }
    }
    if !doc.education.is_empty() {
        h.push_str("<h2>Education</h2>");
        for e in &doc.education {
            let mut line = String::new();
            if !e.degree.trim().is_empty() {
                line.push_str(&esc(&e.degree));
            }
            if !e.field.trim().is_empty() {
                if !line.is_empty() {
                    line.push_str(", ");
                }
                line.push_str(&esc(&e.field));
            }
            h.push_str("<div class=\"entry\">");
            h.push_str(&format!(
                "<div class=\"entry-head\">{}</div>",
                esc(&e.institution)
            ));
            let dates = join_dates(&e.start_date, &e.end_date);
            let mut sub = Vec::new();
            if !line.is_empty() {
                sub.push(line);
            }
            if !dates.is_empty() {
                sub.push(esc(&dates).to_string());
            }
            if !e.grade.trim().is_empty() {
                sub.push(esc(&e.grade).to_string());
            }
            if !sub.is_empty() {
                h.push_str(&format!(
                    "<div class=\"entry-sub\">{}</div>",
                    sub.join(" | ")
                ));
            }
            h.push_str("</div>");
        }
    }
    for (title, items) in [
        ("Certifications", &doc.certifications),
        ("Achievements", &doc.achievements),
    ] {
        if !items.is_empty() {
            h.push_str(&format!("<h2>{title}</h2><ul>"));
            for i in items {
                h.push_str(&format!("<li>{}</li>", esc(i)));
            }
            h.push_str("</ul>");
        }
    }
    if !doc.languages.is_empty() {
        h.push_str(&format!(
            "<h2>Languages</h2><p>{}</p>",
            esc(&doc.languages.join(", "))
        ));
    }
    h.push_str("</div></body></html>");
    h
}

pub fn resume_to_plain_text(doc: &ResumeDocument) -> String {
    let mut t = String::new();
    t.push_str(&doc.contact.name);
    t.push('\n');
    if !doc.headline.is_empty() {
        t.push_str(&doc.headline);
        t.push('\n');
    }
    let contact: Vec<&String> = [
        &doc.contact.location,
        &doc.contact.email,
        &doc.contact.phone,
        &doc.contact.linkedin,
        &doc.contact.github,
        &doc.contact.portfolio,
    ]
    .into_iter()
    .filter(|s| !s.trim().is_empty())
    .collect();
    if !contact.is_empty() {
        t.push_str(
            &contact
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" | "),
        );
        t.push('\n');
    }
    if !doc.summary.is_empty() {
        t.push_str("\nSUMMARY\n");
        t.push_str(&doc.summary);
        t.push('\n');
    }
    if doc.skills.iter().any(|s| !s.items.is_empty()) {
        t.push_str("\nSKILLS\n");
        for s in &doc.skills {
            if !s.items.is_empty() {
                t.push_str(&format!("{}: {}\n", s.name, s.items.join(", ")));
            }
        }
    }
    if !doc.experience.is_empty() {
        t.push_str("\nEXPERIENCE\n");
        for e in &doc.experience {
            t.push_str(&format!(
                "{} - {} ({})\n",
                e.role,
                e.company,
                join_dates(&e.start_date, &e.end_date)
            ));
            for b in &e.bullets {
                t.push_str(&format!("- {b}\n"));
            }
        }
    }
    if !doc.projects.is_empty() {
        t.push_str("\nPROJECTS\n");
        for p in &doc.projects {
            t.push_str(&format!("{} ({})\n", p.name, p.technologies.join(", ")));
            if !p.description.is_empty() {
                t.push_str(&format!("{}\n", p.description));
            }
            for b in &p.bullets {
                t.push_str(&format!("- {b}\n"));
            }
        }
    }
    if !doc.education.is_empty() {
        t.push_str("\nEDUCATION\n");
        for e in &doc.education {
            t.push_str(&format!(
                "{} - {} {} ({})\n",
                e.institution,
                e.degree,
                e.field,
                join_dates(&e.start_date, &e.end_date)
            ));
        }
    }
    if !doc.certifications.is_empty() {
        t.push_str("\nCERTIFICATIONS\n");
        for c in &doc.certifications {
            t.push_str(&format!("- {c}\n"));
        }
    }
    if !doc.achievements.is_empty() {
        t.push_str("\nACHIEVEMENTS\n");
        for a in &doc.achievements {
            t.push_str(&format!("- {a}\n"));
        }
    }
    if !doc.languages.is_empty() {
        t.push_str(&format!("\nLANGUAGES\n{}\n", doc.languages.join(", ")));
    }
    t
}

// ---------------------------------------------------------------------------
// DOCX
// ---------------------------------------------------------------------------

fn heading(text: &str) -> Paragraph {
    Paragraph::new()
        .add_run(Run::new().add_text(text.to_uppercase()).bold().size(22))
        .line_spacing(LineSpacing::new().before(200).after(60))
}

fn plain(text: &str) -> Paragraph {
    Paragraph::new()
        .add_run(Run::new().add_text(text).size(21))
        .line_spacing(LineSpacing::new().after(60))
}

fn bold_line(bold: &str, rest: &str) -> Paragraph {
    let mut p = Paragraph::new().add_run(Run::new().add_text(bold).bold().size(21));
    if !rest.is_empty() {
        p = p.add_run(Run::new().add_text(rest).size(21));
    }
    p.line_spacing(LineSpacing::new().after(20))
}

fn bullet(text: &str) -> Paragraph {
    Paragraph::new()
        .add_run(Run::new().add_text(text).size(21))
        .numbering(NumberingId::new(1), IndentLevel::new(0))
        .line_spacing(LineSpacing::new().after(20))
}

fn base_docx() -> Docx {
    Docx::new()
        .add_abstract_numbering(
            AbstractNumbering::new(1).add_level(
                Level::new(
                    0,
                    Start::new(1),
                    NumberFormat::new("bullet"),
                    LevelText::new("•"),
                    LevelJc::new("left"),
                )
                .indent(
                    Some(360),
                    Some(SpecialIndentType::Hanging(220)),
                    None,
                    None,
                ),
            ),
        )
        .add_numbering(Numbering::new(1, 1))
        .default_fonts(
            RunFonts::new()
                .ascii("Arial")
                .hi_ansi("Arial")
                .east_asia("Arial")
                .cs("Arial"),
        )
        .default_size(21)
}

pub fn resume_to_docx(doc: &ResumeDocument, path: &Path) -> CoreResult<()> {
    let mut d = base_docx();
    d = d.add_paragraph(
        Paragraph::new().add_run(Run::new().add_text(&doc.contact.name).bold().size(40)),
    );
    if !doc.headline.trim().is_empty() {
        d = d.add_paragraph(
            Paragraph::new().add_run(Run::new().add_text(&doc.headline).bold().size(23)),
        );
    }
    let contact: Vec<&str> = [
        &doc.contact.location,
        &doc.contact.email,
        &doc.contact.phone,
        &doc.contact.linkedin,
        &doc.contact.github,
        &doc.contact.portfolio,
    ]
    .into_iter()
    .filter(|s| !s.trim().is_empty())
    .map(|s| s.as_str())
    .collect();
    if !contact.is_empty() {
        d = d.add_paragraph(plain(&contact.join(" | ")));
    }
    if !doc.summary.trim().is_empty() {
        d = d
            .add_paragraph(heading("Summary"))
            .add_paragraph(plain(&doc.summary));
    }
    if doc.skills.iter().any(|s| !s.items.is_empty()) {
        d = d.add_paragraph(heading("Skills"));
        for s in doc.skills.iter().filter(|s| !s.items.is_empty()) {
            d = d.add_paragraph(bold_line(&format!("{}: ", s.name), &s.items.join(", ")));
        }
    }
    if !doc.experience.is_empty() {
        d = d.add_paragraph(heading("Experience"));
        for e in &doc.experience {
            d = d.add_paragraph(bold_line(&format!("{} — {}", e.role, e.company), ""));
            let mut sub = Vec::new();
            let dates = join_dates(&e.start_date, &e.end_date);
            if !dates.is_empty() {
                sub.push(dates);
            }
            if !e.location.trim().is_empty() {
                sub.push(e.location.clone());
            }
            if !sub.is_empty() {
                d = d.add_paragraph(plain(&sub.join(" | ")));
            }
            for b in &e.bullets {
                d = d.add_paragraph(bullet(b));
            }
        }
    }
    if !doc.projects.is_empty() {
        d = d.add_paragraph(heading("Projects"));
        for p in &doc.projects {
            let tech = if p.technologies.is_empty() {
                String::new()
            } else {
                format!(" ({})", p.technologies.join(", "))
            };
            d = d.add_paragraph(bold_line(&p.name, &tech));
            if !p.description.trim().is_empty() {
                d = d.add_paragraph(plain(&p.description));
            }
            for b in &p.bullets {
                d = d.add_paragraph(bullet(b));
            }
            if !p.url.trim().is_empty() {
                d = d.add_paragraph(plain(&p.url));
            }
        }
    }
    if !doc.education.is_empty() {
        d = d.add_paragraph(heading("Education"));
        for e in &doc.education {
            d = d.add_paragraph(bold_line(&e.institution, ""));
            let mut parts = Vec::new();
            let deg = [e.degree.trim(), e.field.trim()]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", ");
            if !deg.is_empty() {
                parts.push(deg);
            }
            let dates = join_dates(&e.start_date, &e.end_date);
            if !dates.is_empty() {
                parts.push(dates);
            }
            if !e.grade.trim().is_empty() {
                parts.push(e.grade.clone());
            }
            if !parts.is_empty() {
                d = d.add_paragraph(plain(&parts.join(" | ")));
            }
        }
    }
    for (title, items) in [
        ("Certifications", &doc.certifications),
        ("Achievements", &doc.achievements),
    ] {
        if !items.is_empty() {
            d = d.add_paragraph(heading(title));
            for i in items {
                d = d.add_paragraph(bullet(i));
            }
        }
    }
    if !doc.languages.is_empty() {
        d = d
            .add_paragraph(heading("Languages"))
            .add_paragraph(plain(&doc.languages.join(", ")));
    }
    let file = std::fs::File::create(path)?;
    d.build()
        .pack(file)
        .map_err(|e| CoreError::DocumentGeneration(format!("DOCX write failed: {e}")))?;
    Ok(())
}

pub fn text_to_docx(text: &str, path: &Path) -> CoreResult<()> {
    let mut d = base_docx();
    for para in text.split("\n\n") {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        let mut p = Paragraph::new();
        for (i, line) in para.lines().enumerate() {
            if i > 0 {
                p = p.add_run(Run::new().add_break(BreakType::TextWrapping));
            }
            p = p.add_run(Run::new().add_text(line).size(22));
        }
        d = d.add_paragraph(p.line_spacing(LineSpacing::new().after(160)));
    }
    let file = std::fs::File::create(path)?;
    d.build()
        .pack(file)
        .map_err(|e| CoreError::DocumentGeneration(format!("DOCX write failed: {e}")))?;
    Ok(())
}

pub fn text_to_html(title: &str, text: &str) -> String {
    let mut h = String::new();
    h.push_str("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>");
    h.push_str(&esc(title));
    h.push_str("</title><style>");
    h.push_str(CSS);
    h.push_str("body { font-size: 11pt; } p { margin: 0 0 10pt 0; white-space: pre-wrap; }</style></head><body><div class=\"page\">");
    for para in text.split("\n\n") {
        if !para.trim().is_empty() {
            h.push_str(&format!("<p>{}</p>", esc(para.trim())));
        }
    }
    h.push_str("</div></body></html>");
    h
}

// ---------------------------------------------------------------------------
// PDF: headless Chrome first, bundled-font renderer as fallback
// ---------------------------------------------------------------------------

async fn html_to_pdf(
    html_path: &Path,
    pdf_path: &Path,
    chrome: Option<&Path>,
    temp_dir: &Path,
    fallback: &(dyn Fn(&Path) -> CoreResult<()> + Send + Sync),
) -> CoreResult<Option<String>> {
    if let Some(chrome) = chrome {
        match crate::chrome::print_to_pdf(chrome, html_path, pdf_path, temp_dir).await {
            Ok(()) => return Ok(None),
            Err(e) => {
                tracing::warn!(error = %e, "Chrome PDF export failed; using built-in renderer");
                let warning = format!("Chrome PDF export failed ({e}); used the built-in renderer");
                fallback(pdf_path)?;
                return Ok(Some(warning));
            }
        }
    }
    fallback(pdf_path)?;
    Ok(Some(
        "Chrome not available; PDF produced with the built-in renderer".into(),
    ))
}

fn font_family() -> CoreResult<genpdf::fonts::FontFamily<genpdf::fonts::FontData>> {
    let load = |bytes: &'static [u8]| {
        genpdf::fonts::FontData::new(bytes.to_vec(), None)
            .map_err(|e| CoreError::DocumentGeneration(format!("font load failed: {e}")))
    };
    Ok(genpdf::fonts::FontFamily {
        regular: load(include_bytes!("../../assets/fonts/DejaVuSans.ttf"))?,
        bold: load(include_bytes!("../../assets/fonts/DejaVuSans-Bold.ttf"))?,
        italic: load(include_bytes!("../../assets/fonts/DejaVuSans-Oblique.ttf"))?,
        bold_italic: load(include_bytes!(
            "../../assets/fonts/DejaVuSans-BoldOblique.ttf"
        ))?,
    })
}

fn genpdf_document(title: &str) -> CoreResult<genpdf::Document> {
    let mut doc = genpdf::Document::new(font_family()?);
    doc.set_title(title);
    doc.set_font_size(10);
    doc.set_line_spacing(1.25);
    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(14);
    doc.set_page_decorator(decorator);
    Ok(doc)
}

pub fn resume_to_pdf_builtin(doc: &ResumeDocument, path: &Path) -> CoreResult<()> {
    use genpdf::elements::{Break, Paragraph, UnorderedList};
    use genpdf::style::Style;
    use genpdf::Element as _;
    let mut pdf = genpdf_document(&format!("{} - Resume", doc.contact.name))?;
    pdf.push(Paragraph::new("").styled_string(
        doc.contact.name.clone(),
        Style::new().bold().with_font_size(18),
    ));
    if !doc.headline.trim().is_empty() {
        pdf.push(
            Paragraph::new("")
                .styled_string(doc.headline.clone(), Style::new().bold().with_font_size(11)),
        );
    }
    let contact: Vec<&str> = [
        &doc.contact.location,
        &doc.contact.email,
        &doc.contact.phone,
        &doc.contact.linkedin,
        &doc.contact.github,
        &doc.contact.portfolio,
    ]
    .into_iter()
    .filter(|s| !s.trim().is_empty())
    .map(|s| s.as_str())
    .collect();
    if !contact.is_empty() {
        pdf.push(Paragraph::new(contact.join(" | ")).styled(Style::new().with_font_size(9)));
    }
    let heading = |pdf: &mut genpdf::Document, text: &str| {
        pdf.push(Break::new(0.6));
        pdf.push(
            Paragraph::new("")
                .styled_string(text.to_uppercase(), Style::new().bold().with_font_size(11)),
        );
    };
    if !doc.summary.trim().is_empty() {
        heading(&mut pdf, "Summary");
        pdf.push(Paragraph::new(doc.summary.clone()));
    }
    if doc.skills.iter().any(|s| !s.items.is_empty()) {
        heading(&mut pdf, "Skills");
        for s in doc.skills.iter().filter(|s| !s.items.is_empty()) {
            pdf.push(
                Paragraph::new("")
                    .styled_string(format!("{}: ", s.name), Style::new().bold())
                    .string(s.items.join(", ")),
            );
        }
    }
    if !doc.experience.is_empty() {
        heading(&mut pdf, "Experience");
        for e in &doc.experience {
            pdf.push(
                Paragraph::new("")
                    .styled_string(format!("{} — {}", e.role, e.company), Style::new().bold()),
            );
            let mut sub = Vec::new();
            let dates = join_dates(&e.start_date, &e.end_date);
            if !dates.is_empty() {
                sub.push(dates);
            }
            if !e.location.trim().is_empty() {
                sub.push(e.location.clone());
            }
            if !sub.is_empty() {
                pdf.push(Paragraph::new(sub.join(" | ")).styled(Style::new().with_font_size(9)));
            }
            if !e.bullets.is_empty() {
                let mut list = UnorderedList::new();
                for b in &e.bullets {
                    list.push(Paragraph::new(b.clone()));
                }
                pdf.push(list);
            }
            pdf.push(Break::new(0.3));
        }
    }
    if !doc.projects.is_empty() {
        heading(&mut pdf, "Projects");
        for p in &doc.projects {
            let tech = if p.technologies.is_empty() {
                String::new()
            } else {
                format!(" ({})", p.technologies.join(", "))
            };
            pdf.push(
                Paragraph::new("")
                    .styled_string(p.name.clone(), Style::new().bold())
                    .string(tech),
            );
            if !p.description.trim().is_empty() {
                pdf.push(Paragraph::new(p.description.clone()));
            }
            if !p.bullets.is_empty() {
                let mut list = UnorderedList::new();
                for b in &p.bullets {
                    list.push(Paragraph::new(b.clone()));
                }
                pdf.push(list);
            }
            pdf.push(Break::new(0.3));
        }
    }
    if !doc.education.is_empty() {
        heading(&mut pdf, "Education");
        for e in &doc.education {
            pdf.push(Paragraph::new("").styled_string(e.institution.clone(), Style::new().bold()));
            let mut parts = Vec::new();
            let deg = [e.degree.trim(), e.field.trim()]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", ");
            if !deg.is_empty() {
                parts.push(deg);
            }
            let dates = join_dates(&e.start_date, &e.end_date);
            if !dates.is_empty() {
                parts.push(dates);
            }
            if !e.grade.trim().is_empty() {
                parts.push(e.grade.clone());
            }
            if !parts.is_empty() {
                pdf.push(Paragraph::new(parts.join(" | ")).styled(Style::new().with_font_size(9)));
            }
        }
    }
    for (title, items) in [
        ("Certifications", &doc.certifications),
        ("Achievements", &doc.achievements),
    ] {
        if !items.is_empty() {
            heading(&mut pdf, title);
            let mut list = UnorderedList::new();
            for i in items {
                list.push(Paragraph::new(i.clone()));
            }
            pdf.push(list);
        }
    }
    if !doc.languages.is_empty() {
        heading(&mut pdf, "Languages");
        pdf.push(Paragraph::new(doc.languages.join(", ")));
    }
    pdf.render_to_file(path)
        .map_err(|e| CoreError::DocumentGeneration(format!("PDF render failed: {e}")))
}

pub fn text_to_pdf_builtin(title: &str, text: &str, path: &Path) -> CoreResult<()> {
    use genpdf::elements::{Break, Paragraph};
    let mut pdf = genpdf_document(title)?;
    pdf.set_font_size(11);
    for para in text.split("\n\n") {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        for line in para.lines() {
            pdf.push(Paragraph::new(line.to_string()));
        }
        pdf.push(Break::new(0.8));
    }
    pdf.render_to_file(path)
        .map_err(|e| CoreError::DocumentGeneration(format!("PDF render failed: {e}")))
}

// ---------------------------------------------------------------------------
// Bundles
// ---------------------------------------------------------------------------

pub struct RenderOptions<'a> {
    pub dir: &'a Path,
    pub base_name: &'a str,
    pub chrome: Option<&'a Path>,
    pub temp_dir: &'a Path,
    pub want_pdf: bool,
    pub want_docx: bool,
}

pub async fn render_resume_bundle(
    doc: &ResumeDocument,
    opts: &RenderOptions<'_>,
) -> CoreResult<RenderedFiles> {
    std::fs::create_dir_all(opts.dir)?;
    let mut files = RenderedFiles::default();
    let json_path: PathBuf = opts.dir.join(format!("{}.json", opts.base_name));
    std::fs::write(&json_path, serde_json::to_string_pretty(doc)?)?;
    files.json = Some(json_path.to_string_lossy().to_string());
    let txt_path = opts.dir.join(format!("{}.txt", opts.base_name));
    std::fs::write(&txt_path, resume_to_plain_text(doc))?;
    files.txt = Some(txt_path.to_string_lossy().to_string());
    let html_path = opts.dir.join(format!("{}.html", opts.base_name));
    std::fs::write(&html_path, resume_to_html(doc))?;
    files.html = Some(html_path.to_string_lossy().to_string());
    if opts.want_docx {
        let docx_path = opts.dir.join(format!("{}.docx", opts.base_name));
        resume_to_docx(doc, &docx_path)?;
        files.docx = Some(docx_path.to_string_lossy().to_string());
    }
    if opts.want_pdf {
        let pdf_path = opts.dir.join(format!("{}.pdf", opts.base_name));
        let d = doc.clone();
        let warning = html_to_pdf(
            &html_path,
            &pdf_path,
            opts.chrome,
            opts.temp_dir,
            &move |p| resume_to_pdf_builtin(&d, p),
        )
        .await?;
        if let Some(w) = warning {
            files.warnings.push(w);
        }
        files.pdf = Some(pdf_path.to_string_lossy().to_string());
    }
    Ok(files)
}

pub async fn render_cover_letter_bundle(
    text: &str,
    title: &str,
    opts: &RenderOptions<'_>,
) -> CoreResult<RenderedFiles> {
    std::fs::create_dir_all(opts.dir)?;
    let mut files = RenderedFiles::default();
    let txt_path = opts.dir.join(format!("{}.txt", opts.base_name));
    std::fs::write(&txt_path, text)?;
    files.txt = Some(txt_path.to_string_lossy().to_string());
    let html_path = opts.dir.join(format!("{}.html", opts.base_name));
    std::fs::write(&html_path, text_to_html(title, text))?;
    files.html = Some(html_path.to_string_lossy().to_string());
    if opts.want_docx {
        let docx_path = opts.dir.join(format!("{}.docx", opts.base_name));
        text_to_docx(text, &docx_path)?;
        files.docx = Some(docx_path.to_string_lossy().to_string());
    }
    if opts.want_pdf {
        let pdf_path = opts.dir.join(format!("{}.pdf", opts.base_name));
        let t = text.to_string();
        let title = title.to_string();
        let warning = html_to_pdf(
            &html_path,
            &pdf_path,
            opts.chrome,
            opts.temp_dir,
            &move |p| text_to_pdf_builtin(&title, &t, p),
        )
        .await?;
        if let Some(w) = warning {
            files.warnings.push(w);
        }
        files.pdf = Some(pdf_path.to_string_lossy().to_string());
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::*;

    fn sample() -> ResumeDocument {
        ResumeDocument {
            contact: ResumeContact {
                name: "Asha Verma".into(),
                email: "asha@example.com".into(),
                phone: "+91 1".into(),
                location: "Mumbai".into(),
                linkedin: "linkedin.com/in/asha".into(),
                ..Default::default()
            },
            headline: "Full Stack Developer".into(),
            summary: "Five years building web apps <with> React & Node.".into(),
            skills: vec![ResumeSkillSection {
                name: "Frontend".into(),
                items: vec!["React".into(), "TypeScript".into()],
            }],
            experience: vec![ResumeExperience {
                company: "Meridian".into(),
                role: "Developer".into(),
                location: "Mumbai".into(),
                start_date: "2022-06".into(),
                end_date: String::new(),
                bullets: vec!["Did things".into(), "Did more".into()],
            }],
            projects: vec![ResumeProject {
                name: "Storefront".into(),
                description: "Rebuilt".into(),
                technologies: vec!["Next.js".into()],
                bullets: vec![],
                url: String::new(),
            }],
            education: vec![ResumeEducation {
                institution: "University".into(),
                degree: "B.E.".into(),
                field: "CS".into(),
                start_date: "2015".into(),
                end_date: "2019".into(),
                grade: "8.4".into(),
            }],
            certifications: vec!["AWS Developer".into()],
            achievements: vec![],
            languages: vec!["English".into()],
        }
    }

    #[test]
    fn html_is_escaped_and_single_column() {
        let html = resume_to_html(&sample());
        assert!(html.contains("&lt;with&gt; React &amp; Node"));
        assert!(!html.contains("<table"));
        assert!(html.contains("<h2>Experience</h2>"));
        assert!(html.contains("2022-06 – Present"));
    }

    #[test]
    fn docx_and_builtin_pdf_render() {
        let dir = tempfile::tempdir().unwrap();
        let docx = dir.path().join("r.docx");
        resume_to_docx(&sample(), &docx).unwrap();
        assert!(docx.metadata().unwrap().len() > 1000);
        let pdf = dir.path().join("r.pdf");
        resume_to_pdf_builtin(&sample(), &pdf).unwrap();
        assert!(pdf.metadata().unwrap().len() > 1000);
        let txt = resume_to_plain_text(&sample());
        assert!(txt.contains("EXPERIENCE"));
    }

    #[tokio::test]
    async fn bundle_without_chrome_uses_builtin_renderer() {
        let dir = tempfile::tempdir().unwrap();
        let opts = RenderOptions {
            dir: dir.path(),
            base_name: "resume-v1",
            chrome: None,
            temp_dir: dir.path(),
            want_pdf: true,
            want_docx: true,
        };
        let files = render_resume_bundle(&sample(), &opts).await.unwrap();
        assert!(
            files.pdf.is_some()
                && files.docx.is_some()
                && files.html.is_some()
                && files.json.is_some()
        );
        assert_eq!(files.warnings.len(), 1);
        let cl = render_cover_letter_bundle(
            "Dear team,\n\nHello.\n\nAsha",
            "Cover letter",
            &RenderOptions {
                base_name: "cover-letter-v1",
                ..opts
            },
        )
        .await
        .unwrap();
        assert!(cl.pdf.is_some() && cl.docx.is_some());
    }
}
