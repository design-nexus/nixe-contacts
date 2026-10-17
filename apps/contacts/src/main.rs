mod control;
mod editor;
mod formats;
mod model;
mod prefs;
mod theme;
mod transfer;
mod ui;
use gtk::{gio, glib, prelude::*};
fn main() -> glib::ExitCode {
    nixe_core::app::init(nixe_core::app::App { slug: "nixe-contacts", name: "Contacts" });
    let args: Vec<_> = std::env::args().collect();
    if args.iter().any(|s| s == "--help" || s == "-h") {
        println!("Usage: nixe-contacts [--section all|favorites|duplicates|settings] [--toggle] [FILE.vcf|FILE.csv …]");
        return glib::ExitCode::SUCCESS;
    }
    if std::env::var_os("GSK_RENDERER").is_none() {
        unsafe {
            std::env::set_var("GSK_RENDERER", "ngl");
        }
    }
    let app = gtk::Application::builder()
        .application_id("io.github.design_nexus.nixe.Contacts")
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    // Nixe Assistant looks people up and adds them through this (primary instance only).
    app.connect_startup(|app| nixe_core::control::serve(app, control::tools(), control::handle));
    app.connect_command_line(|app, cl| {
        let argv: Vec<_> = cl.arguments().iter().map(|s| s.to_string_lossy().into_owned()).collect();
        if argv.iter().any(|s| s == "--toggle") {
            if let Some(w) = app.active_window() {
                w.close();
                return glib::ExitCode::SUCCESS;
            }
        }
        let section = argv.iter().position(|s| s == "--section").and_then(|i| argv.get(i + 1));
        let ui = match ui::present(app, section.map(String::as_str)) {
            Ok(ui) => ui,
            Err(e) => {
                eprintln!("Contacts: {e:#}");
                let (w, b) = nixe_ui::widgets::dialog("Contacts could not open", 480);
                b.append(&gtk::Label::new(Some(&format!("{e:#}\nYour address book has been kept unchanged."))));
                w.set_application(Some(app));
                w.present();
                return glib::ExitCode::FAILURE;
            }
        };
        let mut skip = false;
        for arg in argv.iter().skip(1) {
            if skip {
                skip = false;
                continue;
            }
            if arg == "--section" {
                skip = true;
                continue;
            }
            if arg.starts_with('-') {
                continue;
            }
            let file = cl.create_file_for_arg(arg);
            if let Some(path) = file.path() {
                transfer::import_path(&ui, &path);
            }
        }
        glib::ExitCode::SUCCESS
    });
    app.connect_shutdown(|_| prefs::flush());
    app.run()
}
