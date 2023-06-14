mod imp;

use adw::subclass::prelude::*;
use adw::{prelude::*, *};

use glib::Object;
use gtk::glib::clone;
use gtk::{gio, glib, *};

use crate::components;
use crate::mime_type_entry::MimeTypeEntry;

pub enum MainWindowPage {
    MimeTypes,
    Applications,
}

glib::wrapper! {
    pub struct MainWindow(ObjectSubclass<imp::MainWindow>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl MainWindow {
    pub fn new(app: &adw::Application) -> Self {
        Object::builder().property("application", app).build()
    }

    fn setup_components(&self) {
        match components::Components::new() {
            Ok(components) => self
                .imp()
                .components
                .set(components)
                .expect("Components should only be set once"),
            Err(e) => panic!("Failed to initialize mimeassoc components, error: {}", e),
        }
    }

    fn setup_models(&self) {
        let model = gio::ListStore::new(MimeTypeEntry::static_type());
        self.imp().mime_type_entries.replace(Some(model));

        let selection_model = NoSelection::new(Some(self.mime_type_entries()));
        self.imp().mime_type_entries_list.bind_model(
            Some(&selection_model),
            clone!(@weak self as window => @default-panic, move |obj| {
                let mime_type_entry = obj.downcast_ref::<MimeTypeEntry>().expect("The object should be of type `MimeTypeEntry`.");
                let row = window.create_mime_type_row(mime_type_entry);
                row.upcast()
            }),
        );
    }

    fn mime_type_entries(&self) -> gio::ListStore {
        self.imp()
            .mime_type_entries
            .borrow()
            .clone()
            .expect("Could not get current mime_type_entries.")
    }

    fn create_mime_type_row(&self, entry: &MimeTypeEntry) -> ActionRow {
        // Create row
        let row = ActionRow::builder().build();

        // TODO: We can add the app selector popup as row.add_suffix
        // row.add_prefix(&check_button);

        row.set_title(&entry.mime_type());
        row
    }

    /// Called when everything is set up, and ready to show data
    fn ready(&self) {
        let components = self
            .imp()
            .components
            .get()
            .expect("Expect setup_components() to be called before ready()");

        let mut all_mime_types = components.mime_db.mime_types();
        all_mime_types.sort();

        let mime_type_entries = all_mime_types
            .iter()
            .map(|mt| MimeTypeEntry::new(mt))
            .collect::<Vec<_>>();

        self.mime_type_entries()
            .extend_from_slice(&mime_type_entries);
    }

    fn setup_actions(&self) {
        let action_show_mime_types = gtk::gio::SimpleAction::new("show-mime-types", None);
        action_show_mime_types.connect_activate(clone!(@weak self as window => move |_, _|{
            window.show_page(MainWindowPage::MimeTypes);
        }));
        self.add_action(&action_show_mime_types);

        let action_show_applications = gtk::gio::SimpleAction::new("show-applications", None);
        action_show_applications.connect_activate(clone!(@weak self as window => move |_, _|{
            window.show_page(MainWindowPage::Applications);
        }));
        self.add_action(&action_show_applications);
    }

    fn show_page(&self, page: MainWindowPage) {
        let page_selection_model = self.imp().stack.pages();
        match page {
            MainWindowPage::MimeTypes => {
                println!("Selecting mime types");
                page_selection_model.select_item(0, true);
            }
            MainWindowPage::Applications => {
                println!("Selecting applicaitons");
                page_selection_model.select_item(1, true);
            }
        }
    }
}
