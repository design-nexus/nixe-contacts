//! Contacts' tools on the control interface (`nixe_core::control`), for Nixe Assistant:
//! look people up, add someone, and change or add details. Nothing is removed: no delete,
//! no merge, no clearing a field.

use crate::model::{self, Contact, Field};
use crate::ui;
use nixe_core::control::text;
use serde_json::{json, Value};

pub fn tools() -> Value {
    let list = json!({"type": "array", "items": {"type": "string"}});
    json!([
        {"name": "contacts.find", "description": "Find contacts by name, company, email or phone: their details.", "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}}, "required": ["query"]}},
        {"name": "contacts.add", "description": "Add a new contact.", "inputSchema": {"type": "object", "properties": {"first": {"type": "string"}, "last": {"type": "string"}, "company": {"type": "string"}, "title": {"type": "string"}, "emails": list, "phones": list, "birthday": {"type": "string"}, "notes": {"type": "string"}}, "required": ["first"]}},
        {"name": "contacts.update", "description": "Change a contact's details (by id from contacts.find) or add an email or phone; nothing is removed.", "inputSchema": {"type": "object", "properties": {"id": {"type": "string"}, "first": {"type": "string"}, "last": {"type": "string"}, "company": {"type": "string"}, "title": {"type": "string"}, "birthday": {"type": "string"}, "notes": {"type": "string"}, "add_email": {"type": "string"}, "add_phone": {"type": "string"}, "favorite": {"type": "boolean"}}, "required": ["id"]}},
    ])
}

fn card(c: &Contact) -> Value {
    let values = |f: &[Field]| f.iter().map(|x| json!({"label": x.label, "value": x.value})).collect::<Vec<_>>();
    json!({
        "id": c.id, "name": c.name(false), "company": c.company, "title": c.title, "birthday": c.birthday,
        "emails": values(&c.emails), "phones": values(&c.phones), "favorite": c.favorite,
    })
}

fn field(value: &str) -> Field {
    Field { label: "other".into(), value: value.trim().to_string() }
}

fn short(args: &Value, key: &str) -> Result<Option<String>, String> {
    match args[key].as_str() {
        Some(v) if v.len() > 2000 => Err(format!("{key} is too long")),
        Some(v) => Ok(Some(v.trim().to_string())),
        None => Ok(None),
    }
}

pub fn handle(name: &str, args: Value) -> Result<Value, String> {
    let ui = ui::current().ok_or("Contacts isn't ready")?;
    match name {
        "contacts.find" => {
            let q = text(&args, "query")?;
            let book = ui.book.borrow();
            let found: Vec<Value> = book.contacts.iter().filter(|c| c.matches(&q)).take(10).map(card).collect();
            Ok(json!({"contacts": found, "untrusted_content": true}))
        }
        "contacts.add" => {
            let mut c = Contact { first: text(&args, "first")?.trim().to_string(), ..Contact::default() };
            for (key, slot) in [("last", &mut c.last), ("company", &mut c.company), ("title", &mut c.title), ("birthday", &mut c.birthday), ("notes", &mut c.notes)] {
                if let Some(v) = short(&args, key)? {
                    *slot = v;
                }
            }
            let strings = |key: &str| args[key].as_array().map(|a| a.iter().filter_map(Value::as_str).filter(|s| !s.trim().is_empty()).map(field).collect()).unwrap_or_default();
            c.emails = strings("emails");
            c.phones = strings("phones");
            if !c.valid() {
                return Err("a contact needs a name, company, email or phone".into());
            }
            let mut book = ui.book.borrow().clone();
            if let Some(same) = book.contacts.iter().find(|o| model::duplicates(o, &c)) {
                return Err(format!("{} already has that email or phone (id {})", same.name(false), same.id));
            }
            let added = card(&c);
            book.contacts.push(c);
            if !ui::commit(&ui, book) {
                return Err("couldn't save the address book".into());
            }
            Ok(json!({"added": added}))
        }
        "contacts.update" => {
            let id = text(&args, "id")?;
            let mut book = ui.book.borrow().clone();
            let c = book.contacts.iter_mut().find(|c| c.id == id).ok_or("no contact with that id")?;
            for (key, slot) in [("first", &mut c.first), ("last", &mut c.last), ("company", &mut c.company), ("title", &mut c.title), ("birthday", &mut c.birthday), ("notes", &mut c.notes)] {
                // Change, never clear.
                if let Some(v) = short(&args, key)?.filter(|v| !v.is_empty()) {
                    *slot = v;
                }
            }
            if let Some(e) = short(&args, "add_email")?.filter(|v| !v.is_empty()) {
                c.emails.push(field(&e));
            }
            if let Some(p) = short(&args, "add_phone")?.filter(|v| !v.is_empty()) {
                c.phones.push(field(&p));
            }
            if let Some(f) = args["favorite"].as_bool() {
                c.favorite = f;
            }
            let updated = card(c);
            if !ui::commit(&ui, book) {
                return Err("couldn't save the address book".into());
            }
            Ok(json!({"updated": updated}))
        }
        _ => Err(format!("unknown tool {name}")),
    }
}
