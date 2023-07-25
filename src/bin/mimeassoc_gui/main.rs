mod model;
mod ui;

use gtk::{gdk::Display, glib::*, prelude::*, *};

const APP_ID: &str = "org.zakariya.MimeAssoc";
const APP_VERSION: &str = "0.0.1";
const APP_ICON: &str = "org.zakariya.MimeAssoc";

fn main() -> glib::ExitCode {
    // Register and include resources
    gio::resources_register_include!("mimeassoc.gresource").expect("Failed to register resources.");

    let app = adw::Application::builder().application_id(APP_ID).build();

    app.connect_startup(|app| {
        setup_shortcuts(app);
        load_css();
    });

    app.connect_activate(build_ui);
    app.run()
}

fn load_css() {
    println!("main::load_css");

    let provider = CssProvider::new();
    provider.load_from_resource("/org/zakariya/MimeAssoc/style.css");

    gtk::style_context_add_provider_for_display(
        &Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn setup_shortcuts(app: &adw::Application) {
    println!("main::setup_shortcuts");

    // I presume `app` has an `app.quit` but I couldn't find documentation for it, so make our own
    let action_close = gio::SimpleAction::new("quit", None);
    action_close.connect_activate(clone!(@weak app => move |_, _| {
        app.quit();
    }));
    app.add_action(&action_close);

    // bind accelerators to actions
    app.set_accels_for_action("win.show-mime-types", &["<Ctrl>M"]);
    app.set_accels_for_action("win.show-applications", &["<Ctrl>A"]);
    app.set_accels_for_action("window.close", &["<Ctrl>W"]);
    app.set_accels_for_action("app.quit", &["<Ctrl>Q"]);
}

fn build_ui(app: &adw::Application) {
    println!("main::build_ui");

    let window = ui::MainWindow::new(app);
    //window.show_page(ui::MainWindowPage::Applications);
    window.present();
}
