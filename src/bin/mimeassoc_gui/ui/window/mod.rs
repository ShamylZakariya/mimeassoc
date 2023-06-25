mod imp;

use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use adw::{prelude::*, *};

use glib::Object;
use gtk::glib::clone;
use gtk::{gio, glib, *};

use crate::model::*;
use crate::ui::application_entry_list_row::ApplicationEntryListRow;
use crate::ui::MimeTypeEntryListRow;

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

    fn stores(&self) -> Rc<RefCell<stores::MimeAssocStores>> {
        self.imp()
            .stores
            .get()
            .expect("Expect `setup_models()` to be called before calling `stores()`.")
            .clone()
    }

    fn mime_type_entries(&self) -> gio::ListStore {
        self.imp()
            .mime_type_entries
            .borrow()
            .clone()
            .expect("Could not get mime_type_entries.")
    }

    fn application_entries(&self) -> gio::ListStore {
        self.imp()
            .application_entries
            .borrow()
            .clone()
            .expect("Could not get application_entries.")
    }

    fn setup_models(&self) {
        println!("MainWindow::setup_models");

        // Create models
        match stores::MimeAssocStores::new() {
            Ok(stores) => {
                self.imp()
                    .stores
                    .set(Rc::new(RefCell::new(stores)))
                    .expect("MainWindow::setup_models() should only be set once");
            }
            Err(e) => panic!("Failed to initialize MimeAssocStores, error: {}", e),
        }

        let mime_types_list_store = gio::ListStore::new(MimeTypeEntry::static_type());
        self.imp()
            .mime_type_entries
            .replace(Some(mime_types_list_store));
        self.build_mime_type_entries_list_store();

        let applications_list_store = gio::ListStore::new(ApplicationEntry::static_type());
        self.imp()
            .application_entries
            .replace(Some(applications_list_store));
        self.build_application_entries_list_store();
    }

    /// Populates self::mime_type_entries with the current state of self.stores()
    fn build_mime_type_entries_list_store(&self) {
        let stores = self.stores();
        let apps = &stores.borrow().desktop_entry_store;
        let mime_associations_store = &stores.borrow().mime_associations_store;

        let mut all_mime_types = mime_associations_store.mime_types();
        all_mime_types.sort();

        let mime_type_entries = all_mime_types
            .iter()
            // Hide any mimetypes for which we have no applications
            .filter(|mt| !apps.get_desktop_entries_for_mimetype(mt).is_empty())
            .map(|mt| MimeTypeEntry::new(mt, stores.clone()))
            .collect::<Vec<_>>();

        mime_type_entries
            .iter()
            .for_each(|entry| entry.update_supported_applications());

        self.mime_type_entries()
            .extend_from_slice(&mime_type_entries);
    }

    /// Populates self::application_entries with the current state of self.stores()
    fn build_application_entries_list_store(&self) {
        let stores = self.stores();
        let apps = &stores.borrow().desktop_entry_store;

        let mut all_desktop_entries = apps.desktop_entries();
        all_desktop_entries.sort_by(|a, b| {
            a.name()
                .unwrap_or(&a.id().to_string())
                .cmp(b.name().unwrap_or(&b.id().to_string()))
        });

        let application_entries = all_desktop_entries
            .iter()
            .filter(|de| !de.mime_types().is_empty())
            .map(|de| ApplicationEntry::new(de.id(), stores.clone()))
            .collect::<Vec<_>>();

        self.application_entries()
            .extend_from_slice(&application_entries);
    }

    fn setup_mime_types_pane(&self) {
        println!("MainWindow::setup_mime_types_pane");
        let stores = self.stores();
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
                .expect("The item has to be an `MimeTypeEntry`.");

            let row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<MimeTypeEntryListRow>()
                .expect("The child has to be a `MimeTypeEntryListRow`.");

            row.bind(&mime_type_entry, stores.clone());
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

    fn setup_applications_pane(&self) {
        println!("MainWindow::setup_applications_pane");
        let factory = SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let row = ApplicationEntryListRow::new();
            let list_item = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem");
            list_item.set_child(Some(&row));
        });

        factory.connect_bind(move |_, list_item| {
            let application_entry = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<ApplicationEntry>()
                .expect("The item has to be an `ApplicationEntry`.");

            let row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<ApplicationEntryListRow>()
                .expect("The child has to be a `ApplicationEntryListRow`.");

            row.bind(&application_entry);
        });

        factory.connect_unbind(move |_, list_item| {
            let row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<ApplicationEntryListRow>()
                .expect("The child has to be a `ApplicationEntryListRow`.");
            row.unbind();
        });

        let selection_model = NoSelection::new(Some(self.application_entries()));
        let list_view = ListView::new(Some(selection_model), Some(factory));
        list_view.add_css_class("frame");
        list_view.add_css_class("separators");
        list_view.set_margin_start(12);
        list_view.set_margin_end(12);
        list_view.set_margin_top(12);
        list_view.set_margin_bottom(12);
        self.imp()
            .applications_scrolled_window
            .set_child(Some(&list_view));
    }

    fn setup_actions(&self) {
        println!("MainWindow::setup_actions");

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
                println!("MainWindow::show_page - MimeTypes");
                self.build_mime_type_entries_list_store();
                page_selection_model.select_item(0, true);
            }
            MainWindowPage::Applications => {
                println!("MainWindow::show_page - Applications");
                self.build_application_entries_list_store();
                page_selection_model.select_item(1, true);
            }
        }
    }
}
