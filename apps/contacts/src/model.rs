use anyhow::{Context, Result, bail};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Field {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Contact {
    pub id: String,
    pub first: String,
    pub last: String,
    pub middle: String,
    pub prefix: String,
    pub suffix: String,
    pub display: String,
    pub company: String,
    pub title: String,
    pub birthday: String,
    pub notes: String,
    pub emails: Vec<Field>,
    pub phones: Vec<Field>,
    pub addresses: Vec<Field>,
    pub websites: Vec<Field>,
    pub favorite: bool,
    pub photo: Vec<u8>,
    pub extra: Vec<String>,
}
impl Default for Contact {
    fn default() -> Self {
        Self {
            id: id(),
            first: String::new(),
            last: String::new(),
            middle: String::new(),
            prefix: String::new(),
            suffix: String::new(),
            display: String::new(),
            company: String::new(),
            title: String::new(),
            birthday: String::new(),
            notes: String::new(),
            emails: vec![],
            phones: vec![],
            addresses: vec![],
            websites: vec![],
            favorite: false,
            photo: vec![],
            extra: vec![],
        }
    }
}
impl Contact {
    pub fn name(&self, last_first: bool) -> String {
        if !self.display.is_empty() {
            return self.display.clone();
        }
        let n = if last_first { [self.last.as_str(), self.first.as_str()] } else { [self.first.as_str(), self.last.as_str()] };
        let n = n.into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
        if !n.is_empty() {
            n
        } else if !self.company.is_empty() {
            self.company.clone()
        } else {
            self.emails.first().map(|e| e.value.clone()).unwrap_or_else(|| "Unnamed contact".into())
        }
    }
    pub fn valid(&self) -> bool {
        [&self.first, &self.last, &self.display, &self.company].iter().any(|s| !s.trim().is_empty())
            || !self.emails.is_empty()
            || !self.phones.is_empty()
    }
    pub fn matches(&self, query: &str) -> bool {
        let text = format!(
            "{} {} {} {} {}",
            self.name(false),
            self.first,
            self.last,
            self.company,
            self.emails.iter().chain(&self.phones).map(|f| f.value.as_str()).collect::<Vec<_>>().join(" ")
        )
        .to_lowercase();
        query.split_whitespace().all(|q| {
            text.contains(&q.to_lowercase())
                || (!phone(q).is_empty() && self.phones.iter().any(|p| phone(&p.value).contains(&phone(q))))
        })
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct List {
    pub id: String,
    pub name: String,
    pub members: BTreeSet<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Book {
    pub contacts: Vec<Contact>,
    pub lists: Vec<List>,
}

pub struct Store {
    db: Connection,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let db = Connection::open(path)?;
        let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > 1 {
            bail!("This address book was made by a newer Contacts version.");
        }
        db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS contacts(id TEXT PRIMARY KEY, data TEXT NOT NULL); CREATE TABLE IF NOT EXISTS lists(id TEXT PRIMARY KEY, name TEXT NOT NULL); CREATE TABLE IF NOT EXISTS members(list_id TEXT REFERENCES lists(id) ON DELETE CASCADE, contact_id TEXT REFERENCES contacts(id) ON DELETE CASCADE, PRIMARY KEY(list_id,contact_id)); PRAGMA user_version=1;")?;
        Ok(Self { db })
    }
    pub fn load(&self) -> Result<Book> {
        let mut book = Book::default();
        for row in self.db.prepare("SELECT data FROM contacts ORDER BY rowid")?.query_map([], |r| r.get::<_, String>(0))? {
            book.contacts
                .push(serde_json::from_str(&row?).context("Couldn't read a contact; the database has been kept unchanged")?);
        }
        for row in self
            .db
            .prepare("SELECT id,name FROM lists ORDER BY name COLLATE NOCASE")?
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        {
            let (id, name) = row?;
            let members = self
                .db
                .prepare("SELECT contact_id FROM members WHERE list_id=?")?
                .query_map([&id], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<BTreeSet<_>>>()?;
            book.lists.push(List { id, name, members });
        }
        Ok(book)
    }
    pub fn save(&mut self, book: &Book) -> Result<()> {
        let tx = self.db.transaction()?;
        tx.execute_batch("DELETE FROM members; DELETE FROM lists; DELETE FROM contacts;")?;
        for c in &book.contacts {
            tx.execute("INSERT INTO contacts VALUES(?,?)", params![c.id, serde_json::to_string(c)?])?;
        }
        for l in &book.lists {
            tx.execute("INSERT INTO lists VALUES(?,?)", params![l.id, l.name])?;
            for id in &l.members {
                tx.execute("INSERT INTO members VALUES(?,?)", params![l.id, id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}
pub fn phone(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).collect()
}
pub fn duplicates(a: &Contact, b: &Contact) -> bool {
    a.emails
        .iter()
        .any(|x| !x.value.trim().is_empty() && b.emails.iter().any(|y| x.value.trim().eq_ignore_ascii_case(y.value.trim())))
        || a.phones.iter().any(|x| phone(&x.value).len() >= 7 && b.phones.iter().any(|y| phone(&x.value) == phone(&y.value)))
}
pub fn union_fields(a: &mut Vec<Field>, b: &[Field]) {
    for f in b {
        if !a.contains(f) {
            a.push(f.clone());
        }
    }
}
pub fn merge(book: &mut Book, primary: Contact, other: &str) {
    let target = primary.id.clone();
    book.contacts.retain(|c| c.id != *other && c.id != target);
    book.contacts.push(primary);
    for l in &mut book.lists {
        if l.members.remove(other) {
            l.members.insert(target.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persistence_membership_and_merge() -> Result<()> {
        let mut store = Store::open(Path::new(":memory:"))?;
        let a = Contact {
            first: "Ada".into(),
            emails: vec![Field { label: "work".into(), value: "ada@example.org".into() }],
            ..Default::default()
        };
        let b = Contact {
            first: "Ada".into(),
            emails: vec![Field { label: "home".into(), value: "ADA@example.org".into() }],
            ..Default::default()
        };
        assert!(duplicates(&a, &b));
        assert!(!duplicates(&a, &Contact { first: "Ada".into(), ..Default::default() }));
        let mut book = Book {
            contacts: vec![a.clone(), b.clone()],
            lists: vec![List { id: id(), name: "Work".into(), members: [b.id.clone()].into() }],
        };
        store.save(&book)?;
        assert_eq!(store.load()?, book);
        merge(&mut book, a.clone(), &b.id);
        store.save(&book)?;
        assert_eq!(store.load()?.contacts, vec![a.clone()]);
        assert!(store.load()?.lists[0].members.contains(&a.id));
        let mut broken = book.clone();
        broken.lists[0].members.insert("missing".into());
        assert!(store.save(&broken).is_err());
        assert_eq!(store.load()?, book);
        Ok(())
    }
}
