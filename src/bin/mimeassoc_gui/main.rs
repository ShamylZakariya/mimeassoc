mod components;
mod window;

use gtk::{gdk::Display, glib::clone, prelude::*, *};

const APP_ID: &str = "org.zakariya.MimeAssoc";

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
    println!("load_css");

    let provider = CssProvider::new();
    provider.load_from_resource("/org/zakariya/MimeAssoc/style.css");

    gtk::style_context_add_provider_for_display(
        &Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn setup_shortcuts(app: &adw::Application) {
    println!("setup_shortcuts");

    app.set_accels_for_action("win.show-mime-types", &["<Ctrl>M"]);
    app.set_accels_for_action("win.show-applications", &["<Ctrl>A"]);

    // window close
    app.set_accels_for_action("window.close", &["<Ctrl>W"]);
    app.set_accels_for_action("win.quit", &["<Ctrl>Q"]);
}

fn build_ui(app: &adw::Application) {
    println!("build_ui");

    let window = window::MainWindow::new(app);

    let action_close = gio::SimpleAction::new("quit", None);
    action_close.connect_activate(clone!(@weak window => move |_, _| {
        window.close();
    }));
    window.add_action(&action_close);

    window.present();
}
