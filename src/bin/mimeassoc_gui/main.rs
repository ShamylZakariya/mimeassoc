mod common;
mod model;
mod ui;

use gtk::{gdk::Display, glib::*, prelude::*, *};
use mimeassoc::MimeType;

///////////////////////////////////////////////////////////////////////////////

struct StdoutLogger;

impl log::Log for StdoutLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let use_glib_logging = false;
            if use_glib_logging {
                // glib logging needs a domain for filtering
                static GLIB_LOG_DOMAIN: &str = "MimeAssoc";

                let level = match record.level() {
                    log::Level::Error => glib::LogLevel::Error,
                    log::Level::Warn => glib::LogLevel::Warning,
                    log::Level::Info => glib::LogLevel::Info,
                    log::Level::Debug => glib::LogLevel::Debug,
                    log::Level::Trace => glib::LogLevel::Debug, // glib doesn't have a "trace" level
                };

                // this almost works; we get output and correct log levels, but
                // the g_log! macro inserts this module (mimeassoc_gui) as the source,
                // not the module where the trace was created (record.target()).
                // TODO: Work out how to write directly to glib logging APIs.
                if let Some(message) = record.args().as_str() {
                    g_log!(GLIB_LOG_DOMAIN, level, "{}", message);
                } else {
                    g_log!(GLIB_LOG_DOMAIN, level, "{}", record.args().to_string());
                }
            } else {
                println!(
                    "{} - {}: {}",
                    record.level(),
                    record.target(),
                    record.args()
                );
            }
        }
    }

    fn flush(&self) {}
}

fn set_logger() {
    static LOGGER: StdoutLogger = StdoutLogger;

    // check "LOG_LEVEL_FILTER" env var. It can be an int corresponding to level (0,1,2,3,4,5), or a string e.g.,
    // "Error", "Warn", etc. If empty, default to log level of Off
    let log_level_filter = if let Ok(log_level_filter_str) = std::env::var("LOG_LEVEL_FILTER") {
        if let Ok(log_level_filter) = log_level_filter_str.parse() {
            match log_level_filter {
                5 => log::LevelFilter::Trace,
                4 => log::LevelFilter::Debug,
                3 => log::LevelFilter::Info,
                2 => log::LevelFilter::Warn,
                1 => log::LevelFilter::Error,
                _ => log::LevelFilter::Off,
            }
        } else {
            let log_level_filter_str = log_level_filter_str.trim().to_lowercase();
            match log_level_filter_str.as_str() {
                "all" | "trace" => log::LevelFilter::Trace,
                "debug" => log::LevelFilter::Debug,
                "info" => log::LevelFilter::Info,
                "warn" => log::LevelFilter::Warn,
                "error" => log::LevelFilter::Error,
                _ => log::LevelFilter::Off,
            }
        }
    } else {
        log::LevelFilter::Off
    };

    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log_level_filter))
        .expect("Expect to set up logger");
}

///////////////////////////////////////////////////////////////////////////////

fn main() -> glib::ExitCode {
    // get logging set up and load resources
    set_logger();
    gio::resources_register_include!("mimeassoc.gresource").expect("Failed to register resources.");

    let app = adw::Application::builder()
        .application_id(crate::common::APP_ID)
        .build();

    app.connect_startup(|app| {
        setup_shortcuts(app);
        load_css();
    });

    app.connect_activate(build_ui);
    app.run()
}

fn load_css() {
    log::debug!("main::load_css");

    let provider = CssProvider::new();
    provider.load_from_resource("/org/zakariya/MimeAssoc/style.css");

    gtk::style_context_add_provider_for_display(
        &Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn setup_shortcuts(app: &adw::Application) {
    log::debug!("main::setup_shortcuts");

    // I presume `app` has an `app.quit` but I couldn't find documentation for it, so make our own
    let action_close = gio::SimpleAction::new("quit", None);
    action_close.connect_activate(clone!(@weak app => move |_, _| {
        app.quit();
    }));
    app.add_action(&action_close);

    // bind accelerators to actions
    app.set_accels_for_action("win.show-mime-types", &["<Ctrl>M"]);
    app.set_accels_for_action("win.show-applications", &["<Ctrl>A"]);
    app.set_accels_for_action("win.undo", &["<Ctrl>Z"]);
    app.set_accels_for_action("win.log-history-stack", &["<Ctrl><Shift>H"]);
    app.set_accels_for_action("window.close", &["<Ctrl>W"]);
    app.set_accels_for_action("app.quit", &["<Ctrl>Q"]);
}

fn build_ui(app: &adw::Application) {
    log::debug!("main::build_ui");

    let window = ui::MainWindow::new(app);

    let mime_type = MimeType::parse("application/vnd.lotus-1-2-3").unwrap();
    window.perform_command(ui::MainWindowCommand::ShowMimeType(mime_type));

    window.present();
}
