mod imp;

use adw::subclass::prelude::*;
use adw::*;

use glib::Object;
use gtk::{gio, glib};

use crate::components;

glib::wrapper! {
    pub struct MainWindow(ObjectSubclass<imp::MainWindow>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl MainWindow {
    pub fn new(app: &gtk::Application) -> Self {
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
}
