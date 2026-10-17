# Contacts — notes for working on this repo

- GTK4 (gtk4-rs 0.11) + Rust, no libadwaita, built to `STYLE.md`. App id
  `io.github.design_nexus.nixe.Contacts`; data in `~/.local/share/nixe-contacts`.
- `model.rs`: `Contact`, `Book`, and `Store` (SQLite, one JSON row per contact, lists and
  members). `ui.rs` holds the window state (`State`, `ui::current()`) and saves through
  `ui::commit(ui, book)`, which refreshes the window.
- **Control** (`control.rs`): `nixe_core::control` on the app's bus name, for Nixe
  Assistant: `contacts.find`, `add` (refused for a duplicate email or phone), `update`
  (change or add, never remove).
- Checks: `cargo test`. `cargo clippy --all-targets -- -D warnings` still reports older
  warnings in `editor.rs`, `transfer.rs`, `ui.rs` and `main.rs`.
- Test runs: point `XDG_DATA_HOME` at a scratch dir so the real address book isn't touched;
  `dbus-run-session --` keeps a running copy out of it.
