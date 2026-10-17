use crate::{
    model::{self, Contact, Field},
    prefs, theme,
    ui::{self, Ui},
};
use gtk::{gdk, glib, prelude::*};
use nixe_ui::widgets as w;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct Fields {
    rows: FieldRows,
}
impl Fields {
    fn new(title: &str, values: &[Field], body: &gtk::Box) -> Self {
        body.append(&ui::label(title, "contact-heading"));
        let root = w::vbox(6);
        body.append(&root);
        let rows = Rc::new(RefCell::new(vec![]));
        for f in values {
            add_field(&root, &rows, f);
        }
        let add = ui::button("+ Add", "Add another labeled value");
        body.append(&add);
        let r = root.clone();
        let rs = rows.clone();
        add.connect_clicked(move |_| add_field(&r, &rs, &Field { label: "other".into(), value: String::new() }));
        Self { rows }
    }
    fn values(&self) -> Vec<Field> {
        self.rows
            .borrow()
            .iter()
            .filter(|(b, _, v)| {
                b.parent().is_some()
                    && !v.buffer().text(&v.buffer().start_iter(), &v.buffer().end_iter(), false).trim().is_empty()
            })
            .map(|(_, l, v)| Field {
                label: l.text().trim().to_string(),
                value: v.buffer().text(&v.buffer().start_iter(), &v.buffer().end_iter(), false).trim().to_string(),
            })
            .collect()
    }
}
/// A field editor's rows: the row, its label entry and its value.
type FieldRows = Rc<RefCell<Vec<(gtk::Box, gtk::Entry, gtk::TextView)>>>;
/// The merge card's choices: which field, its picker, and the values offered.
type Choices = Rc<RefCell<Vec<(usize, gtk::DropDown, Vec<String>)>>>;
fn add_field(root: &gtk::Box, rows: &FieldRows, f: &Field) {
    let row = w::hbox(6);
    let name = gtk::Entry::new();
    name.set_width_chars(8);
    name.set_max_width_chars(9);
    name.set_placeholder_text(Some("Label"));
    name.set_text(&f.label);
    name.set_valign(gtk::Align::Start);
    row.append(&name);
    let value = gtk::TextView::new();
    value.set_hexpand(true);
    value.set_wrap_mode(gtk::WrapMode::WordChar);
    value.set_top_margin(6);
    value.set_bottom_margin(6);
    value.set_left_margin(6);
    value.set_right_margin(6);
    value.buffer().set_text(&f.value);
    value.add_css_class("contact-input");
    row.append(&value);
    let remove = w::bar_button("list-remove-symbolic", "Remove field");
    remove.set_valign(gtk::Align::Start);
    row.append(&remove);
    let r = root.clone();
    let b = row.clone();
    remove.connect_clicked(move |_| r.remove(&b));
    root.append(&row);
    rows.borrow_mut().push((row, name, value));
}
const NAMES: &[&str] =
    &["First name", "Last name", "Middle name", "Prefix", "Suffix", "Display name", "Company", "Job title", "Birthday"];
fn singles(c: &Contact) -> Vec<String> {
    vec![
        c.first.clone(),
        c.last.clone(),
        c.middle.clone(),
        c.prefix.clone(),
        c.suffix.clone(),
        c.display.clone(),
        c.company.clone(),
        c.title.clone(),
        c.birthday.clone(),
    ]
}
fn set_singles(c: &mut Contact, v: &[String]) {
    c.first = v[0].clone();
    c.last = v[1].clone();
    c.middle = v[2].clone();
    c.prefix = v[3].clone();
    c.suffix = v[4].clone();
    c.display = v[5].clone();
    c.company = v[6].clone();
    c.title = v[7].clone();
    c.birthday = v[8].clone();
}
pub fn open(ui: &Ui, contact: Option<Contact>) {
    open_inner(ui, contact, None);
}
fn open_inner(ui: &Ui, contact: Option<Contact>, merge_other: Option<String>) {
    if let Some(e) = ui.editor.borrow().as_ref() {
        e.present();
        return;
    }
    let is_new = contact.is_none();
    let original = contact.unwrap_or_default();
    let c = original.clone();
    let (dialog, card) = w::dialog(
        if merge_other.is_some() {
            "Review merged contact"
        } else if is_new {
            "New contact"
        } else {
            "Edit contact"
        },
        600,
    );
    dialog.set_default_height(720);
    dialog.set_transient_for(Some(&ui.window));
    dialog.set_application(ui.window.application().as_ref());
    *ui.editor.borrow_mut() = Some(dialog.clone());
    let body = w::vbox(8);
    let scroll = ui::scroll(&body);
    scroll.set_min_content_height(320);
    card.append(&scroll);
    let photo = Rc::new(RefCell::new(c.photo.clone()));
    let preview = w::hbox(0);
    preview.append(&ui::avatar(&c, 72));
    body.append(&preview);
    let photo_bar = w::hbox(8);
    let choose = ui::button("Choose photo…", "Choose a PNG or JPEG photo");
    let remove = ui::button("Remove photo", "Use initials instead of a photo");
    photo_bar.append(&choose);
    photo_bar.append(&remove);
    body.append(&photo_bar);
    let p = photo.clone();
    let box_ = preview.clone();
    let mut no_photo = c.clone();
    no_photo.photo.clear();
    remove.connect_clicked(move |_| {
        p.borrow_mut().clear();
        ui::clear(&box_);
        box_.append(&ui::avatar(&no_photo, 72));
    });
    let p = photo.clone();
    let parent = dialog.clone();
    let preview = preview.clone();
    let c_photo = c.clone();
    let u = ui.clone();
    choose.connect_clicked(move |_| {
        let picker = gtk::FileDialog::builder().title("Choose contact photo").modal(true).build();
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("PNG and JPEG photos"));
        filter.add_mime_type("image/png");
        filter.add_mime_type("image/jpeg");
        picker.set_default_filter(Some(&filter));
        let p = p.clone();
        let preview = preview.clone();
        let c = c_photo.clone();
        let u = u.clone();
        picker.open(Some(&parent), None::<&gtk::gio::Cancellable>, move |result| {
            if let Ok(file) = result
                && let Some(path) = file.path() {
                    let load = || -> anyhow::Result<Vec<u8>> {
                        if path.metadata()?.len() > 10 * 1024 * 1024 {
                            anyhow::bail!("Photos must be smaller than 10 MB.");
                        }
                        let bytes = std::fs::read(path)?;
                        let t = gdk::Texture::from_bytes(&glib::Bytes::from(&bytes[..]))?;
                        if t.width() > 8192 || t.height() > 8192 {
                            anyhow::bail!("Photo dimensions must be at most 8192 × 8192.");
                        }
                        Ok(bytes)
                    };
                    match load() {
                        Ok(bytes) => {
                            *p.borrow_mut() = bytes.clone();
                            let mut c = c.clone();
                            c.photo = bytes;
                            ui::clear(&preview);
                            preview.append(&ui::avatar(&c, 72));
                        }
                        Err(e) => ui::toast(&u, &e.to_string()),
                    }
                }
        });
    });
    let entries: Vec<_> = NAMES
        .iter()
        .zip(singles(&c))
        .map(|(name, value)| {
            body.append(&ui::label(name, "dim"));
            let entry = gtk::Entry::new();
            entry.set_text(&value);
            if *name == "Birthday" {
                entry.set_placeholder_text(Some("YYYY-MM-DD or --MM-DD"));
            }
            body.append(&entry);
            entry
        })
        .collect();
    let favorite = gtk::CheckButton::with_label("Favorite");
    favorite.set_active(c.favorite);
    body.append(&favorite);
    let emails = Fields::new("EMAIL", &c.emails, &body);
    let phones = Fields::new("PHONE", &c.phones, &body);
    let addresses = Fields::new("ADDRESS", &c.addresses, &body);
    let websites = Fields::new("WEBSITE", &c.websites, &body);
    body.append(&ui::label("NOTES", "contact-heading"));
    let notes = gtk::TextView::new();
    notes.set_wrap_mode(gtk::WrapMode::WordChar);
    notes.set_height_request(100);
    notes.buffer().set_text(&c.notes);
    notes.add_css_class("contact-input");
    body.append(&notes);
    let error = ui::label("", "error");
    error.set_visible(false);
    card.append(&error);
    let buttons = w::hbox(8);
    buttons.set_halign(gtk::Align::End);
    let cancel = ui::button("Cancel", "Close editor (Esc)");
    let save = ui::button("Save", "Save contact (Ctrl+S)");
    save.add_css_class("suggested-action");
    buttons.append(&cancel);
    buttons.append(&save);
    card.append(&buttons);
    let gather: Rc<dyn Fn() -> Contact> = Rc::new(move || {
        let mut c = c.clone();
        set_singles(&mut c, &entries.iter().map(|e| e.text().trim().to_string()).collect::<Vec<_>>());
        c.favorite = favorite.is_active();
        c.photo = photo.borrow().clone();
        c.emails = emails.values();
        c.phones = phones.values();
        c.addresses = addresses.values();
        c.websites = websites.values();
        c.notes = notes.buffer().text(&notes.buffer().start_iter(), &notes.buffer().end_iter(), false).to_string();
        c
    });
    let saved = Rc::new(Cell::new(false));
    let dirty = Rc::new(Cell::new(false));
    let prompt = Rc::new(Cell::new(false));
    let g = gather.clone();
    let baseline = original.clone();
    let s = saved.clone();
    let u = ui.clone();
    let d = dialog.clone();
    let asking = prompt.clone();
    let discard = dirty.clone();
    dialog.connect_close_request(move |_| {
        if !s.get() && !discard.get() && g() != baseline {
            if !asking.replace(true) {
                let ask = gtk::AlertDialog::builder()
                    .message("Discard unsaved changes?")
                    .detail("Your changes have not been saved.")
                    .buttons(["Keep editing", "Discard"])
                    .cancel_button(0)
                    .default_button(0)
                    .modal(true)
                    .build();
                let d = d.clone();
                let disc = discard.clone();
                let asking = asking.clone();
                ask.choose(Some(&d.clone()), None::<&gtk::gio::Cancellable>, move |choice| {
                    asking.set(false);
                    if choice == Ok(1) {
                        disc.set(true);
                        d.close();
                    }
                });
            }
            return glib::Propagation::Stop;
        }
        u.editor.borrow_mut().take();
        glib::Propagation::Proceed
    });
    let d = dialog.clone();
    cancel.connect_clicked(move |_| d.close());
    let u = ui.clone();
    let d = dialog.clone();
    let run: Rc<dyn Fn()> = Rc::new(move || {
        let c = gather();
        if !c.valid() {
            error.set_text("Enter a name, company, email, or phone number.");
            error.set_visible(true);
            return;
        }
        if !valid_birthday(&c.birthday) {
            error.set_text("Use a valid birthday: YYYY-MM-DD or --MM-DD.");
            error.set_visible(true);
            return;
        }
        let mut book = u.book.borrow().clone();
        if let Some(other) = &merge_other {
            model::merge(&mut book, c.clone(), other);
        } else {
            if let Some(old) = book.contacts.iter_mut().find(|old| old.id == c.id) {
                *old = c.clone();
            } else {
                book.contacts.push(c.clone());
            }
            if is_new {
                let section = u.section.borrow().clone();
                if let Some(list) = book.lists.iter_mut().find(|l| l.id == section) {
                    list.members.insert(c.id.clone());
                }
            }
        }
        if ui::commit(&u, book) {
            *u.selected.borrow_mut() = vec![c.id.clone()];
            u.showing_detail.set(true);
            ui::refresh(&u);
            saved.set(true);
            d.close();
        } else {
            error.set_text("Could not save the contact. Check the main window for details.");
            error.set_visible(true);
        }
    });
    let r = run.clone();
    save.connect_clicked(move |_| r());
    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(move |_, key, _, mods| {
        if mods.contains(gdk::ModifierType::CONTROL_MASK) && matches!(key, gdk::Key::s | gdk::Key::S) {
            run();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    dialog.add_controller(keys);
    dialog.present();
}
fn valid_birthday(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    let s = if let Some(rest) = s.strip_prefix("--") { format!("2000-{rest}") } else { s.into() };
    let parts: Vec<_> = s.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return false;
    }
    let (Ok(y), Ok(m), Ok(d)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>(), parts[2].parse::<u32>()) else { return false };
    if y == 0 || m == 0 || m > 12 {
        return false;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let max = match m {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    d > 0 && d <= max
}

pub fn review_merge(ui: &Ui, a: &Contact, b: &Contact) {
    let (dialog, card) = w::dialog("Review duplicate", 580);
    dialog.set_default_height(640);
    card.append(&ui::label("Choose the primary card, then review conflicting values. Distinct emails, phones, addresses, websites, notes, and list memberships will be kept.","dim"));
    let primary = gtk::DropDown::from_strings(&[&format!("Primary: {}", a.name(false)), &format!("Primary: {}", b.name(false))]);
    card.append(&primary);
    let form = w::vbox(8);
    card.append(&ui::scroll(&form));
    let choices: Choices = Rc::new(RefCell::new(vec![]));
    let build: Rc<dyn Fn(bool)> = {
        let form = form.clone();
        let choices = choices.clone();
        let a = a.clone();
        let b = b.clone();
        Rc::new(move |reverse| {
            ui::clear(&form);
            choices.borrow_mut().clear();
            let (p, o) = if reverse { (&b, &a) } else { (&a, &b) };
            for (i, (x, y)) in singles(p).into_iter().zip(singles(o)).enumerate() {
                if !x.is_empty() && !y.is_empty() && x != y {
                    form.append(&ui::label(NAMES[i], "dim"));
                    let values = vec![x, y];
                    let dd = gtk::DropDown::from_strings(&values.iter().map(String::as_str).collect::<Vec<_>>());
                    form.append(&dd);
                    choices.borrow_mut().push((i, dd, values));
                }
            }
        })
    };
    build(false);
    let f = build.clone();
    primary.connect_selected_notify(move |p| f(p.selected() == 1));
    let photo_choice = gtk::DropDown::from_strings(&["Use primary photo", "Use other photo"]);
    card.append(&photo_choice);
    let button = ui::button("Continue to editor", "Review the combined contact before saving");
    card.append(&button);
    let u = ui.clone();
    let a = a.clone();
    let b = b.clone();
    let d = dialog.clone();
    button.connect_clicked(move |_| {
        let (mut p, o) = if primary.selected() == 1 { (b.clone(), a.clone()) } else { (a.clone(), b.clone()) };
        let mut single = singles(&p);
        for (x, y) in single.iter_mut().zip(singles(&o)) {
            if x.is_empty() {
                *x = y;
            }
        }
        for (i, dd, values) in choices.borrow().iter() {
            single[*i] = values[dd.selected() as usize].clone();
        }
        set_singles(&mut p, &single);
        model::union_fields(&mut p.emails, &o.emails);
        model::union_fields(&mut p.phones, &o.phones);
        model::union_fields(&mut p.addresses, &o.addresses);
        model::union_fields(&mut p.websites, &o.websites);
        if !o.notes.is_empty() && p.notes != o.notes {
            if !p.notes.is_empty() {
                p.notes.push_str("\n\n");
            }
            p.notes.push_str(&o.notes);
        }
        p.favorite |= o.favorite;
        if photo_choice.selected() == 1 || p.photo.is_empty() {
            p.photo = o.photo.clone();
        }
        for extra in &o.extra {
            if !p.extra.contains(extra) && !extra.to_uppercase().starts_with("UID:") {
                p.extra.push(extra.clone());
            }
        }
        d.close();
        open_inner(&u, Some(p), Some(o.id));
    });
    dialog.present();
}

pub fn settings(ui: &Ui) {
    let p = prefs::get();
    let body = w::vbox(0);
    body.add_css_class("settings-page");
    body.set_hexpand(true);
    let root = ui::scroll(&body);
    fn group(body: &gtk::Box, name: &str) -> gtk::Box {
        let outer = w::vbox(0);
        outer.add_css_class("settings-group");
        outer.set_widget_name(name);
        outer.append(&ui::label(&name.to_uppercase(), "group-title"));
        let rows = w::vbox(0);
        rows.add_css_class("group-list");
        outer.append(&rows);
        body.append(&outer);
        rows
    }
    let g = group(&body, "Contacts");
    for (title, desc, value, sort) in [
        ("Display last name first", "Show family names before given names.", p.last_first, false),
        ("Sort by last name", "Order the contact list by family name.", p.sort_last, true),
    ] {
        let u = ui.clone();
        let (r, _) = w::switch_row(title, desc, value, move |on| {
            prefs::update(|p| if sort { p.sort_last = on } else { p.last_first = on });
            ui::refresh(&u);
        });
        g.append(&r);
    }
    let g = group(&body, "Contacts window");
    g.append(&w::start_row());
    let (row, dd) = w::choice_row(
        "Theme",
        "Choose a bundled or user theme.",
        theme::all().into_iter().map(|t| (t.id, t.name)).collect(),
        &p.theme,
        |id| {
            prefs::update(|p| {
                p.mode = prefs::ThemeMode::Theme;
                p.theme = id;
            });
            theme::apply();
        },
    );
    dd.set_sensitive(p.mode == prefs::ThemeMode::Theme || !theme::omarchy_available());
    if theme::omarchy_available() {
        let (r, _) = w::switch_row(
            "Follow Omarchy theme",
            "Match the desktop colors and update live.",
            p.mode == prefs::ThemeMode::Omarchy,
            move |on| {
                prefs::update(|p| p.mode = if on { prefs::ThemeMode::Omarchy } else { prefs::ThemeMode::Theme });
                dd.set_sensitive(!on);
                theme::apply();
            },
        );
        g.append(&r);
    }
    g.append(&row);
    let swatches = w::hbox(4);
    let repaint = {
        let s = swatches.clone();
        move || {
            ui::clear(&s);
            let p = theme::current_palette();
            for color in [&p.bg, &p.surface, &p.muted, &p.text, &p.accent, &p.danger] {
                let b = w::hbox(0);
                b.set_size_request(24, 24);
                b.add_css_class("swatch");
                let css = gtk::CssProvider::new();
                css.load_from_string(&format!("box {{ background: {color}; }}"));
                #[allow(deprecated)]
                b.style_context().add_provider(&css, gtk::STYLE_PROVIDER_PRIORITY_USER);
                s.append(&b);
            }
        }
    };
    repaint();
    let weak = swatches.downgrade();
    glib::timeout_add_seconds_local(1, move || {
        if weak.upgrade().is_none() {
            return glib::ControlFlow::Break;
        }
        repaint();
        glib::ControlFlow::Continue
    });
    g.append(&w::row("Current colors", "", Some(swatches.upcast_ref())));
    for (name, on, motion) in [("Glow", p.glow, false), ("Reduce motion", p.reduce_motion, true)] {
        let (r, _) = w::switch_row(name, "", on, move |on| {
            prefs::update(|p| if motion { p.reduce_motion = on } else { p.glow = on });
            theme::apply();
        });
        g.append(&r);
    }
    let g = group(&body, "Keyboard");
    for (keys, action) in ui::SHORTCUTS {
        g.append(&w::row(action, "", Some(w::key_caps(keys).upcast_ref())));
    }
    nixe_ui::settings_card::open_page(
        &ui.overlay,
        &ui.window,
        nixe_ui::settings_card::Spec {
            groups: &[
                ("Contacts", "system-users-symbolic", "Names and sorting"),
                ("Contacts window", "applications-graphics-symbolic", "Theme and motion"),
                ("Keyboard", "input-keyboard-symbolic", "Shortcuts"),
            ],
            search_text: None,
        },
        root.upcast_ref(),
        &body,
    );
}
