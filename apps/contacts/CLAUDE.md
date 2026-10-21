# Contacts — notes for working on this repo

- GTK4 (gtk4-rs 0.11) + Rust, no libadwaita, built to `STYLE.md`. App id
  `io.github.design_nexus.nixe.Contacts`; data in `~/.local/share/nixe-contacts`.
- `model.rs`: `Contact`, `Book`, and `Store` (SQLite, one JSON row per contact, lists and
  members). `Store::save` writes only the contact rows whose JSON changed (lists and
  members whole). Photos serialize as base64 and still read the old number arrays.
  `Address` is the seven vCard parts joined by newlines in `Field::value` (a value with
  another count is a free-form street); never trim that value as a whole.
  `normalize_birthday` is the one birthday parser (import, editor, control). `DupIndex`
  finds shared emails/phones once per book; `ui::dup_ids` caches it until `commit`.
- `ui.rs` holds the window state (`State`, `ui::current()`) and saves through
  `ui::commit(ui, book)`, which refreshes the window. The editor saves only the fields it
  changed onto the contact as it is now (`changes_onto`).
- `formats.rs`: vCard/CSV, independent of GTK. Labels come from every TYPE (`label_of`);
  raw lines kept in `extra` are matched by `property_of`; `uid` reads the card's UID, which
  import uses to update rather than duplicate (`transfer::existing`).
- **Control** (`control.rs`): `nixe_core::control` on the app's bus name, for Nixe
  Assistant: `contacts.find`, `birthdays`, `lists`, `add` (refused for a duplicate email
  or phone), `update` (change or add, never remove; `notes` appends), `add_to_list`. New
  tools also go in the Assistant's `abilities/nixe-apps/plugin.toml` and, if they change
  anything, `assistant-policy`'s capability and summary.
- Online books (`eds:` sections) come from `nixe-accounts` and are read-only.
- Checks: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- Test runs: point `XDG_DATA_HOME` at a scratch dir so the real address book isn't touched;
  `dbus-run-session --` keeps a running copy out of it.
