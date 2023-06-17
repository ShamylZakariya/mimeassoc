mod imp;

use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use adw::{prelude::*, *};

use glib::Object;
use gtk::glib::clone;
use gtk::{gio, glib, *};

use crate::application_entry::ApplicationEntry;
use crate::components;
use crate::mime_type_entry::MimeTypeEntry;
use crate::mime_type_entry_list_row::MimeTypeEntryListRow;

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

    fn components(&self) -> Rc<RefCell<components::Components>> {
        self.imp()
            .components
            .get()
            .expect("Expect components to have been set")
            .clone()
    }

    fn mime_type_entries(&self) -> gio::ListStore {
        self.imp()
            .mime_type_entries
            .borrow()
            .clone()
            .expect("Could not get current mime_type_entries.")
    }

    fn setup_models(&self) {
        // Create models
        match components::Components::new() {
            Ok(components) => {
                self.imp()
                    .components
                    .set(Rc::new(RefCell::new(components)))
                    .expect("MainWindow::setup_models() should only be set once");
            }
            Err(e) => panic!("Failed to initialize mimeassoc components, error: {}", e),
        }

        let model = gio::ListStore::new(MimeTypeEntry::static_type());
        self.imp().mime_type_entries.replace(Some(model));

        // Populate the mime type model
        let components = self.components();
        let app_db = &components.borrow().app_db;
        let mime_db = &components.borrow().mime_db;

        let mut all_mime_types = mime_db.mime_types();
        all_mime_types.sort();

        let mime_type_entries = all_mime_types
            .iter()
            // Hide any mimetypes for which we have no applications
            .filter(|mt| !app_db.get_desktop_entries_for_mimetype(mt).is_empty())
            .map(|mt| MimeTypeEntry::new(mt, &components.borrow()))
            .collect::<Vec<_>>();

        self.mime_type_entries()
            .extend_from_slice(&mime_type_entries);
    }

    fn setup_mime_types_pane(&self) {
        let components = self.components();
        let factory = SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let row = MimeTypeEntryListRow::new();
            let list_item = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem");
            list_item.set_child(Some(&row));
        });

        factory.connect_bind(move |_, list_item| {
            let mime_type_entry = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<MimeTypeEntry>()
                .expect("The item has to be an `ApplicationEntry`.");

            let row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<MimeTypeEntryListRow>()
                .expect("The child has to be a `MimeTypeEntryListRow`.");

            row.bind(&mime_type_entry, components.clone());
        });

        factory.connect_unbind(move |_, list_item| {
            let row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<MimeTypeEntryListRow>()
                .expect("The child has to be a `MimeTypeEntryListRow`.");
            row.unbind();
        });

        let selection_model = NoSelection::new(Some(self.mime_type_entries()));
        let list_view = ListView::new(Some(selection_model), Some(factory));
        list_view.add_css_class("frame");
        list_view.add_css_class("separators");
        list_view.set_margin_start(12);
        list_view.set_margin_end(12);
        list_view.set_margin_top(12);
        list_view.set_margin_bottom(12);
        self.imp()
            .mime_types_scrolled_window
            .set_child(Some(&list_view));
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
