//! File interchange stays independent of GTK so previews and round trips share one parser.
use crate::model::{Contact, Field};
use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};

#[derive(Default)]
pub struct Import {
    pub contacts: Vec<Contact>,
    pub warnings: Vec<String>,
}
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n").replace(';', "\\;").replace(',', "\\,").replace('\r', "")
}
fn unesc(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(c) = chars.next() {
                out.push(if c == 'n' || c == 'N' { '\n' } else { c });
            }
        } else {
            out.push(c);
        }
    }
    out
}
fn parts(s: &str) -> Vec<String> {
    let mut v = vec![String::new()];
    let mut escaped = false;
    for c in s.chars() {
        if c == ';' && !escaped {
            v.push(String::new());
        } else {
            v.last_mut().unwrap().push(c);
        }
        if c == '\\' {
            escaped = !escaped;
        } else {
            escaped = false;
        }
    }
    v.iter().map(|s| unesc(s)).collect()
}
pub fn vcard_read(text: &str) -> Import {
    let mut lines: Vec<String> = vec![];
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim_end_matches('\r');
        if (line.starts_with(' ') || line.starts_with('\t')) && !lines.is_empty() {
            lines.last_mut().unwrap().push_str(&line[1..]);
        } else {
            lines.push(line.into());
        }
    }
    let mut result = Import::default();
    let mut card: Option<Contact> = None;
    let mut version = false;
    let mut bad = false;
    for line in lines {
        if line.eq_ignore_ascii_case("BEGIN:VCARD") {
            if card.is_some() {
                result.warnings.push("Skipped an unfinished contact card.".into());
            }
            card = Some(Contact::default());
            version = false;
            bad = false;
            continue;
        }
        if line.eq_ignore_ascii_case("END:VCARD") {
            if let Some(mut c) = card.take() {
                let display = std::mem::take(&mut c.display);
                if display != c.name(false) {
                    c.display = display;
                }
                if version && !bad && c.valid() {
                    result.contacts.push(c);
                } else {
                    result
                        .warnings
                        .push("Skipped a malformed, empty, or unsupported contact card (vCard 3.0/4.0 required).".into());
                }
            }
            continue;
        }
        let Some(c) = card.as_mut() else { continue };
        let Some((key, value)) = line.split_once(':') else {
            bad = true;
            continue;
        };
        let mut key_parts = key.split(';');
        let property = key_parts.next().unwrap().rsplit('.').next().unwrap().to_uppercase();
        let params: Vec<_> = key_parts.collect();
        let label = params
            .iter()
            .find_map(|p| {
                p.split_once('=').filter(|(k, _)| k.eq_ignore_ascii_case("TYPE")).map(|(_, v)| v.trim_matches('"').to_lowercase())
            })
            .unwrap_or_else(|| "other".into());
        let field = Field { label, value: unesc(value) };
        match property.as_str() {
            "VERSION" => version = value == "3.0" || value == "4.0",
            "UID" => {} // Imported cards always get a local identity; their UID is retained separately.
            "FN" => c.display = unesc(value),
            "N" => {
                let p = parts(value);
                c.last = p.first().cloned().unwrap_or_default();
                c.first = p.get(1).cloned().unwrap_or_default();
                c.middle = p.get(2).cloned().unwrap_or_default();
                c.prefix = p.get(3).cloned().unwrap_or_default();
                c.suffix = p.get(4).cloned().unwrap_or_default();
            }
            "ORG" => {
                let p = parts(value);
                c.company = p.first().cloned().unwrap_or_default();
                if p.len() > 1 {
                    c.extra.push(line.clone());
                }
            }
            "TITLE" => c.title = unesc(value),
            "BDAY" => c.birthday = unesc(value),
            "NOTE" => c.notes = unesc(value),
            "EMAIL" => c.emails.push(field),
            "TEL" => c.phones.push(Field { value: field.value.trim_start_matches("tel:").to_string(), ..field }),
            "URL" => c.websites.push(field),
            "ADR" => c.addresses.push(Field { value: parts(value).join("\n"), ..field }),
            "PHOTO" => {
                let encoded = value.split_once(";base64,").map(|(_, s)| s).or_else(|| {
                    params
                        .iter()
                        .any(|p| p.eq_ignore_ascii_case("ENCODING=b") || p.eq_ignore_ascii_case("ENCODING=BASE64"))
                        .then_some(value)
                });
                if let Some(data) = encoded {
                    match STANDARD.decode(data) {
                        Ok(bytes) => c.photo = bytes,
                        Err(_) => {
                            result
                                .warnings
                                .push("A contact photo could not be decoded; its original property was retained.".into());
                            c.extra.push(line.clone());
                        }
                    }
                } else {
                    c.extra.push(line.clone());
                }
            }
            "X-NIXE-FAVORITE" => c.favorite = value == "1",
            _ => c.extra.push(line.clone()),
        }
        if property == "UID" {
            c.extra.push(line);
        }
    }
    if card.is_some() {
        result.warnings.push("Skipped an unfinished contact card.".into());
    }
    if result.contacts.is_empty() && result.warnings.is_empty() {
        result.warnings.push("No vCard contacts found.".into());
    }
    result
}
fn fold(line: &str) -> String {
    let mut out = String::new();
    let mut width = 0;
    for c in line.chars() {
        if width + c.len_utf8() > 75 {
            out.push_str("\r\n ");
            width = 1;
        }
        out.push(c);
        width += c.len_utf8();
    }
    out.push_str("\r\n");
    out
}
pub fn vcard_write(contacts: &[Contact]) -> String {
    let mut out = String::new();
    for c in contacts {
        let mut lines = vec![
            "BEGIN:VCARD".into(),
            "VERSION:3.0".into(),
            format!("FN:{}", esc(&c.name(false))),
            format!("N:{};{};{};{};{}", esc(&c.last), esc(&c.first), esc(&c.middle), esc(&c.prefix), esc(&c.suffix)),
        ];
        if !c.extra.iter().any(|s| s.to_uppercase().starts_with("UID:")) {
            lines.push(format!("UID:{}", c.id));
        }
        for (key, v) in [("ORG", &c.company), ("TITLE", &c.title), ("BDAY", &c.birthday), ("NOTE", &c.notes)] {
            if !v.is_empty() {
                lines.push(format!("{key}:{}", esc(v)));
            }
        }
        for (key, fields) in [("EMAIL", &c.emails), ("TEL", &c.phones), ("URL", &c.websites), ("ADR", &c.addresses)] {
            for f in fields {
                let label: String = f.label.chars().filter(|c| c.is_ascii_alphanumeric() || *c == ',' || *c == '-').collect();
                let value = if key == "ADR" {
                    let p: Vec<_> = f.value.split('\n').collect();
                    if p.len() == 7 {
                        p.into_iter().map(esc).collect::<Vec<_>>().join(";")
                    } else {
                        format!(";;{};;;;", esc(&f.value))
                    }
                } else {
                    esc(&f.value)
                };
                lines.push(format!("{key};TYPE={}:{}", if label.is_empty() { "other" } else { &label }, value));
            }
        }
        if !c.photo.is_empty() {
            let mime = if c.photo.starts_with(b"\x89PNG") { "PNG" } else { "JPEG" };
            lines.push(format!("PHOTO;ENCODING=b;TYPE={mime}:{}", STANDARD.encode(&c.photo)));
        }
        if c.favorite {
            lines.push("X-NIXE-FAVORITE:1".into());
        }
        // ORG may carry departments; retain it only when it still agrees with the edited company.
        for extra in &c.extra {
            if extra.to_uppercase().starts_with("ORG:") {
                if parts(extra.split_once(':').unwrap().1).first() == Some(&c.company) {
                    lines.retain(|l| !l.starts_with("ORG:"));
                    lines.push(extra.clone());
                }
            } else {
                lines.push(extra.clone());
            }
        }
        lines.push("END:VCARD".into());
        for line in lines {
            out.push_str(&fold(&line));
        }
    }
    out
}

pub const COLUMNS: &[&str] = &[
    "First Name",
    "Last Name",
    "Middle Name",
    "Title",
    "Suffix",
    "Company",
    "Job Title",
    "Birthday",
    "Notes",
    "E-mail Address",
    "E-mail 2 Address",
    "E-mail 3 Address",
    "Mobile Phone",
    "Home Phone",
    "Business Phone",
    "Home Street",
    "Business Street",
    "Web Page",
];
pub struct CsvData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub warnings: Vec<String>,
}
pub fn csv_data(text: &str) -> Result<CsvData> {
    let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(text.trim_start_matches('\u{feff}').as_bytes());
    let headers = reader.headers()?.iter().map(str::to_string).collect();
    let mut rows = vec![];
    let mut warnings = vec![];
    for (i, row) in reader.records().enumerate() {
        match row {
            Ok(row) => rows.push(row.iter().map(str::to_string).collect()),
            Err(e) => warnings.push(format!("CSV row {}: {e}", i + 2)),
        }
    }
    Ok(CsvData { headers, rows, warnings })
}
pub fn mapping(headers: &[String]) -> Vec<Option<usize>> {
    headers
        .iter()
        .map(|h| {
            let normalized = h.to_lowercase().replace([' ', '_', '-'], "");
            COLUMNS.iter().position(|c| c.to_lowercase().replace([' ', '-'], "") == normalized).or(match normalized.as_str() {
                "givenname" => Some(0),
                "surname" | "familyname" => Some(1),
                "organization" => Some(5),
                "email" | "emailaddress" => Some(9),
                "phone" | "telephone" => Some(12),
                "website" => Some(17),
                _ => None,
            })
        })
        .collect()
}
pub fn csv_contacts(data: &CsvData, map: &[Option<usize>]) -> Import {
    let mut out = Import { warnings: data.warnings.clone(), ..Default::default() };
    for (i, row) in data.rows.iter().enumerate() {
        let mut c = Contact::default();
        for (value, column) in row.iter().zip(map) {
            if value.trim().is_empty() {
                continue;
            }
            match column {
                Some(0) => c.first = value.clone(),
                Some(1) => c.last = value.clone(),
                Some(2) => c.middle = value.clone(),
                Some(3) => c.prefix = value.clone(),
                Some(4) => c.suffix = value.clone(),
                Some(5) => c.company = value.clone(),
                Some(6) => c.title = value.clone(),
                Some(7) => c.birthday = value.clone(),
                Some(8) => c.notes = value.clone(),
                Some(9..=11) => c.emails.push(Field { label: "other".into(), value: value.clone() }),
                Some(12..=14) => c.phones.push(Field {
                    label: match column {
                        Some(12) => "cell",
                        Some(13) => "home",
                        _ => "work",
                    }
                    .into(),
                    value: value.clone(),
                }),
                Some(15..=16) => c
                    .addresses
                    .push(Field { label: if *column == Some(15) { "home" } else { "work" }.into(), value: value.clone() }),
                Some(17) => c.websites.push(Field { label: "other".into(), value: value.clone() }),
                _ => {}
            }
        }
        if c.valid() {
            out.contacts.push(c);
        } else {
            out.warnings.push(format!("CSV row {} has no mapped contact identity and was skipped.", i + 2));
        }
    }
    out
}
pub fn csv_write(contacts: &[Contact]) -> Result<Vec<u8>> {
    let mut w = csv::Writer::from_writer(vec![]);
    w.write_record(COLUMNS)?;
    for c in contacts {
        let mut row = vec![String::new(); COLUMNS.len()];
        for (i, s) in [
            (0, &c.first),
            (1, &c.last),
            (2, &c.middle),
            (3, &c.prefix),
            (4, &c.suffix),
            (5, &c.company),
            (6, &c.title),
            (7, &c.birthday),
            (8, &c.notes),
        ] {
            row[i] = s.clone();
        }
        if row[0].is_empty() && row[1].is_empty() {
            row[0] = c.name(false);
        }
        for (i, f) in c.emails.iter().take(3).enumerate() {
            row[9 + i] = f.value.clone();
        }
        for f in &c.phones {
            let i = match f.label.as_str() {
                "cell" | "mobile" => 12,
                "work" => 14,
                _ => 13,
            };
            if !row[i].is_empty() {
                row[i].push_str("; ");
            }
            row[i].push_str(&f.value);
        }
        for f in &c.addresses {
            let i = if f.label == "work" { 16 } else { 15 };
            if !row[i].is_empty() {
                row[i].push('\n');
            }
            row[i].push_str(&f.value);
        }
        row[17] = c.websites.first().map(|f| f.value.clone()).unwrap_or_default();
        w.write_record(row)?;
    }
    Ok(w.into_inner()?)
}
pub fn read(path: &std::path::Path) -> Result<(String, bool)> {
    let meta = path.metadata()?;
    if meta.len() > 50 * 1024 * 1024 {
        bail!("Import files must be smaller than 50 MB.");
    }
    Ok((std::fs::read_to_string(path)?, path.extension().is_some_and(|e| e.eq_ignore_ascii_case("csv"))))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vcard_roundtrip_and_malformed() {
        let text = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Zoë Example\r\nN:Example;Zoë;;;\r\nEMAIL;TYPE=work:zoe@example.org\r\nNOTE:Line 1\\nLine 2\\;yes\r\nX-CUSTOM:preserve me\r\nEND:VCARD\r\nBEGIN:VCARD\r\nVERSION:3.0\r\nFN:Unfinished";
        let i = vcard_read(text);
        assert_eq!(i.contacts.len(), 1);
        assert_eq!(i.warnings.len(), 1);
        let mut c = i.contacts[0].clone();
        c.notes.push_str(&"🙂".repeat(80));
        c.photo = vec![1, 2, 3];
        let encoded = vcard_write(&[c.clone()]);
        assert!(encoded.lines().all(|l| l.len() <= 75));
        let next = vcard_read(&encoded);
        assert_eq!(next.contacts[0].notes, c.notes);
        assert_eq!(next.contacts[0].photo, c.photo);
        assert!(next.contacts[0].extra.iter().any(|s| s == "X-CUSTOM:preserve me"));
    }
    #[test]
    fn csv_quotes_mapping_and_roundtrip() -> Result<()> {
        let data = csv_data("Given Name,Surname,Email,Notes\r\nAda,Lovelace,ada@example.org,\"Hello, world\nNext line\"\r\n,,,")?;
        let parsed = csv_contacts(&data, &mapping(&data.headers));
        assert_eq!(parsed.contacts.len(), 1);
        assert_eq!(parsed.warnings.len(), 1);
        let bytes = csv_write(&parsed.contacts)?;
        let data = csv_data(std::str::from_utf8(&bytes)?)?;
        let next = csv_contacts(&data, &mapping(&data.headers));
        assert_eq!(next.contacts[0].notes, "Hello, world\nNext line");
        assert_eq!(next.contacts[0].emails, parsed.contacts[0].emails);
        Ok(())
    }
}
