use gtk::{prelude::*, *};
use mimeassoc::desktop_entry::*;
use mimeassoc::mime_type::*;
use mimeassoc::*;

const APP_ID: &str = "org.zakariya.MimeAssoc";

struct MimeAssocState {
    mime_db: MimeAssociations,
    app_db: DesktopEntries,
}

impl MimeAssocState {
    fn new() -> anyhow::Result<Self> {
        let desktop_entry_dirs = match desktop_entry_dirs() {
            Ok(desktop_entry_dirs) => desktop_entry_dirs,
            Err(e) => panic!("Unable to load desktop_entry_dirs: {:?}", e),
        };

        let mimeapps_lists = match mimeapps_lists_paths() {
            Ok(mimeapps_lists) => mimeapps_lists,
            Err(e) => panic!("Unable to load mimeapps_lists_paths: {:?}", e),
        };

        let mime_db = match MimeAssociations::load(&mimeapps_lists) {
            Ok(mimeassoc) => mimeassoc,
            Err(e) => panic!("Unable to load MimeAssociations: {:?}", e),
        };

        let app_db = match DesktopEntries::load(&desktop_entry_dirs) {
            Ok(desktop_entries) => desktop_entries,
            Err(e) => panic!("Unable to load DesktopEntries: {:?}", e),
        };

        Ok(Self { mime_db, app_db })
    }
}

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let button_increase = Button::builder()
        .label("Assign image/png to Photopea")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    button_increase.connect_clicked(move |_| {
        // We probably need to ... subclass Application and give it a MimeAssocState?
        match MimeAssocState::new() {
            Ok(mut mimeassoc) => {
                let Some(photopea) = lookup_desktop_entry(&mimeassoc.app_db, "Photopea") else {
                    println!("Unable to find Photopea in apps lists");
                    return;
                };
                let image_png = MimeType::parse("image/png").unwrap();
                if let Err(e) = mimeassoc
                    .mime_db
                    .set_default_handler_for_mime_type(&image_png, &photopea)
                {
                    println!(
                        "Unable to assign photopea to handle image/png. Error: {:?}",
                        e
                    );
                }
            }
            Err(e) => {
                println!("Unable to create mimeassoc, error: {:?}", e);
            }
        }
    });

    let gtk_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .build();

    gtk_box.append(&button_increase);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Mime Assoc")
        .child(&gtk_box)
        .build();

    window.present();
}
