mod imp;

use adw::subclass::prelude::*;
use adw::{prelude::*, *};

use glib::Object;
use gtk::glib::clone;
use gtk::{gio, glib};

use crate::components;

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

    pub fn show_page(&self, page: MainWindowPage) {
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
