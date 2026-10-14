# Contacts

A local address book for the Nixe Suite, built with Rust, GTK4, and the suite's shared theme and widgets.

Install with `./install.sh contacts` from the repository root. Launch `nixe-contacts`, or open a `.vcf` file to preview an import. No account or network service is required.

Create contacts with labeled email addresses, phones, postal addresses, websites, birthday, notes, and photos. Organize them into lists or favorites. Use the toolbar menu for bulk membership changes, deletion with Undo, list email, and import/export. Review matching emails or phone numbers in Duplicates; merges require review and preserve distinct fields and list memberships.

Data lives in `$XDG_DATA_HOME/nixe-contacts/contacts.db` (normally `~/.local/share/nixe-contacts/contacts.db`). Photos are embedded in the app-owned database; originals remain untouched. Preferences live in `$XDG_CONFIG_HOME/nixe-contacts/settings.toml`. Back up the database while Contacts is closed to preserve contacts, lists, photos, and favorites together.

Import supports UTF-8 vCard 3.0/4.0 and CSV with a column mapping screen. vCard export uses version 3.0 and preserves unknown properties. CSV uses common Outlook headers, including three email columns, mobile/home/business phones, home/business street addresses, and a website. Multiple phone numbers in a category share one CSV cell; vCard is preferable for lossless multi-value exchange. CSV does not preserve photos, lists, favorites, or custom properties. vCard exports contact details and favorites, but not list membership. Remote photo URLs are retained without downloading them.

`nixe-contacts --section all|favorites|duplicates|settings`, `--toggle`, and `--help` follow suite conventions. Search filters the current view. Export uses the selection, or the entire current filtered view when nothing is selected.

| Shortcut | Action |
| --- | --- |
| Ctrl+N | New contact |
| Ctrl+S | Save contact in the editor |
| Ctrl+F | Search contacts |
| Ctrl+B | Collapse sidebar |
| Ctrl+, | Settings |
| Ctrl+Shift+P | Command palette |
| Esc | Close search/card or return to list |
| F1 / ? | Shortcuts |
| Ctrl+Q / Ctrl+W | Close window |

Account synchronization, directories, smart lists, and mail-history integration are outside this local release.
