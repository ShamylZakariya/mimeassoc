mod components;
mod window;

use gtk::{gdk::Display, prelude::*, *};

const APP_ID: &str = "org.zakariya.MimeAssoc";

fn main() -> glib::ExitCode {
    // Register and include resources
    gio::resources_register_include!("mimeassoc.gresource").expect("Failed to register resources.");

    let app = gtk::Application::builder().application_id(APP_ID).build();

    app.connect_startup(|app| {
        setup_shortcuts(app);
        load_css();
    });

    app.connect_activate(build_ui);
    app.run()
}

fn load_css() {
    println!("load_css");

    let provider = CssProvider::new();
    provider.load_from_resource("/org/zakariya/MimeAssoc/style.css");

    gtk::style_context_add_provider_for_display(
        &Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn setup_shortcuts(app: &gtk::Application) {
    println!("setup_shortcuts");
    app.set_accels_for_action("window.close", &["<Ctrl>W"]);
    app.set_accels_for_action("window.close", &["<Ctrl>Q"]);
}

fn build_ui(app: &gtk::Application) {
    println!("build_ui");

    let window = window::MainWindow::new(app);
    window.present();
}
