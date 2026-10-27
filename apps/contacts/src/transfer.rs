use crate::{
    formats::{self, Import},
    model,
    ui::{self, Ui},
};
use gtk::{gio, prelude::*};
use nyx_ui::widgets as w;
use std::{path::Path, rc::Rc};
pub fn choose_import(ui: &Ui) {
    let picker = gtk::FileDialog::builder().title("Import contacts").modal(true).build();
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("vCard and CSV files"));
    for pattern in ["*.vcf", "*.vcard", "*.csv", "*.VCF", "*.CSV"] {
        filter.add_pattern(pattern);
    }
    picker.set_default_filter(Some(&filter));
    let u = ui.clone();
    picker.open(Some(&ui.window), None::<&gio::Cancellable>, move |result| {
        if let Ok(file) = result
            && let Some(path) = file.path() {
                import_path(&u, &path);
            }
    });
}
pub fn import_path(ui: &Ui, path: &Path) {
    match formats::read(path) {
        Ok((text, csv)) => {
            if csv {
                match formats::csv_data(&text) {
                    Ok(data) => mapping(ui, data),
                    Err(e) => ui::toast(ui, &format!("Could not read CSV: {e}")),
                }
            } else {
                preview(ui, formats::vcard_read(&text));
            }
        }
        Err(e) => ui::toast(ui, &format!("Could not import: {e}")),
    }
}
fn mapping(ui: &Ui, data: formats::CsvData) {
    let (dialog, card) = w::dialog("Map CSV columns", 560);
    dialog.set_default_height(620);
    card.append(&ui::label("Review the destination for each column. Unmapped columns will be omitted.", "dim"));
    let body = w::vbox(8);
    card.append(&ui::scroll(&body));
    let mut options = vec!["Skip column".to_string()];
    options.extend(formats::COLUMNS.iter().map(|s| s.to_string()));
    let refs: Vec<_> = options.iter().map(String::as_str).collect();
    let defaults = formats::mapping(&data.headers);
    let mut choices = vec![];
    for (header, default) in data.headers.iter().zip(defaults) {
        body.append(&ui::label(header, "dim"));
        let dd = gtk::DropDown::from_strings(&refs);
        dd.set_selected(default.map(|i| i as u32 + 1).unwrap_or(0));
        body.append(&dd);
        choices.push(dd);
    }
    let error = ui::label("", "error");
    card.append(&error);
    let b = ui::button("Preview import", "Review contacts before importing");
    card.append(&b);
    let u = ui.clone();
    let d = dialog.clone();
    b.connect_clicked(move |_| {
        let map: Vec<_> = choices.iter().map(|dd| (dd.selected() as usize).checked_sub(1)).collect();
        let mut used = std::collections::BTreeSet::new();
        if map.iter().flatten().any(|i| !used.insert(*i)) {
            error.set_text("Map each destination field only once.");
            return;
        }
        let parsed = formats::csv_contacts(&data, &map);
        d.close();
        preview(&u, parsed);
    });
    dialog.present();
}
/// The book contact an imported card is a newer copy of: same vCard UID, or a UID
/// that is the contact's own id (a card exported from here).
fn existing<'a>(book: &'a model::Book, c: &model::Contact) -> Option<&'a model::Contact> {
    let uid = formats::uid(c)?;
    book.contacts.iter().find(|o| o.id == uid || formats::uid(o).as_deref() == Some(uid.as_str()))
}

fn preview(ui: &Ui, import: Import) {
    let (dialog, card) = w::dialog("Import preview", 580);
    dialog.set_default_height(600);
    let book = ui.book.borrow();
    let updates = import.contacts.iter().filter(|c| existing(&book, c).is_some()).count();
    // Duplicates among the new cards, and against the book, through one index.
    let mut all = book.contacts.clone();
    all.extend(import.contacts.iter().filter(|c| existing(&book, c).is_none()).cloned());
    let index = model::DupIndex::new(&all);
    let flagged = index.flagged();
    let duplicates = all[book.contacts.len()..].iter().filter(|c| flagged.contains(c.id.as_str())).count();
    drop(book);
    card.append(&ui::label(
        &format!(
            "{} contacts · {} update existing ones · {} warnings · {} possible duplicates",
            import.contacts.len(),
            updates,
            import.warnings.len(),
            duplicates
        ),
        "",
    ));
    if duplicates > 0 {
        card.append(&ui::label("Duplicates are imported as separate cards. Review them in Duplicates after importing.", "dim"));
    }
    let body = w::vbox(8);
    card.append(&ui::scroll(&body));
    for warning in &import.warnings {
        body.append(&ui::label(warning, "error"));
    }
    for c in &import.contacts {
        body.append(&ui::label(
            &format!("{} — {}", c.name(false), c.emails.first().map(|f| f.value.as_str()).unwrap_or(&c.company)),
            "",
        ));
    }
    let actions = w::hbox(8);
    actions.set_halign(gtk::Align::End);
    let cancel = ui::button("Cancel", "Cancel import");
    let d = dialog.clone();
    cancel.connect_clicked(move |_| d.close());
    actions.append(&cancel);
    let save = ui::button("Import", "Add the previewed contacts to the address book");
    save.add_css_class("suggested-action");
    save.set_sensitive(!import.contacts.is_empty());
    actions.append(&save);
    card.append(&actions);
    let u = ui.clone();
    let d = dialog.clone();
    save.connect_clicked(move |_| {
        let mut book = u.book.borrow().clone();
        let section = u.section.borrow().clone();
        for c in &import.contacts {
            let mut c = c.clone();
            // A newer copy of a card already here replaces it, keeping its id and lists.
            if let Some(id) = existing(&book, &c).map(|o| o.id.clone()) {
                c.id = id.clone();
                if let Some(old) = book.contacts.iter_mut().find(|o| o.id == id) {
                    c.favorite |= old.favorite;
                    *old = c;
                }
                continue;
            }
            c.id = model::id();
            if let Some(l) = book.lists.iter_mut().find(|l| l.id == section) {
                l.members.insert(c.id.clone());
            }
            book.contacts.push(c);
        }
        if ui::commit(&u, book) {
            d.close();
            ui::toast(&u, &format!("Imported {} contacts.", import.contacts.len()));
        }
    });
    dialog.present();
}
pub fn export(ui: &Ui, csv: bool) {
    let contacts = ui::chosen(ui);
    if contacts.is_empty() {
        ui::toast(ui, "There are no contacts to export.");
        return;
    }
    if csv {
        let (dialog, card) = w::dialog("Export CSV", 520);
        card.append(&ui::label("CSV includes names, company, job title, birthday, notes, up to three emails, phone and street address columns, and the first website. Photos, list membership, custom properties, and additional websites are omitted. Use vCard for richer contact details.","dim"));
        let b = ui::button("Choose destination…", "Save an Outlook-compatible CSV");
        card.append(&b);
        let u = ui.clone();
        let contacts = contacts.clone();
        let d = dialog.clone();
        b.connect_clicked(move |_| {
            d.close();
            save_export(&u, contacts.clone(), true);
        });
        dialog.present();
    } else {
        save_export(ui, contacts, false);
    }
}
fn save_export(ui: &Ui, contacts: Vec<model::Contact>, csv: bool) {
    let data = if csv { formats::csv_write(&contacts) } else { Ok(formats::vcard_write(&contacts).into_bytes()) };
    let bytes = match data {
        Ok(bytes) => Rc::new(bytes),
        Err(e) => {
            ui::toast(ui, &format!("Could not export: {e}"));
            return;
        }
    };
    let picker = gtk::FileDialog::builder()
        .title("Export contacts")
        .initial_name(if csv { "Contacts.csv" } else { "Contacts.vcf" })
        .modal(true)
        .build();
    let u = ui.clone();
    picker.save(Some(&ui.window), None::<&gio::Cancellable>, move |result| {
        if let Ok(file) = result
            && let Some(path) = file.path() {
                match nyx_core::cmd::atomic_write(&path, &*bytes) {
                    Ok(()) => ui::toast(&u, &format!("Exported {} contacts.", contacts.len())),
                    Err(e) => ui::toast(&u, &format!("Could not export: {e}")),
                }
            }
    });
}
