mod imp;

use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use adw::{prelude::*, *};

use glib::Object;
use gtk::glib::clone;
use gtk::{gio, glib, *};

use crate::model::*;
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

    fn stores(&self) -> Rc<RefCell<MimeAssocStores>> {
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
        match MimeAssocStores::new() {
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
        let mime_associations_store = &stores.borrow().mime_associations_store;

        let mut all_mime_types = mime_associations_store.mime_types();
        all_mime_types.sort();

        let mime_type_entries = all_mime_types
            .iter()
            .map(|mt| MimeTypeEntry::new(mt, stores.clone()))
            .filter(|e| e.supported_application_entries().n_items() > 0)
            .collect::<Vec<_>>();

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
            let model = list_item
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

            row.bind(&model, stores.clone());
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
        self.imp()
            .mime_types_scrolled_window
            .set_child(Some(&list_view));
    }

    fn setup_applications_pane(&self) {
        println!("MainWindow::setup_applications_pane");

        // Populate applications list
        self.imp().application_list_box.bind_model(
            Some(&self.application_entries()),
            clone!(@weak self as window => @default-panic, move |obj| {
                let model = obj
                    .downcast_ref()
                    .unwrap();
                let row = window.create_application_list_box_row(model);
                row.upcast()
            }),
        );

        // Listen for selection
        self.imp().application_list_box.connect_row_activated(
            clone!(@weak self as window => move |_, row|{
                let index = row.index();
                let model = window.application_entries().item(index as u32)
                    .expect("Expected valid item index")
                    .downcast::<ApplicationEntry>()
                    .expect("MainWindow::application_entries should only contain ApplicationEntry");
                window.show_application_mime_type_assignment(&model);
            }),
        );

        // Select first entry
        let row = self.imp().application_list_box.row_at_index(0);
        self.imp().application_list_box.select_row(row.as_ref());
        self.show_application_mime_type_assignment(
            &self
                .application_entries()
                .item(0)
                .expect("Expect non-empty application entries model")
                .downcast::<ApplicationEntry>()
                .expect("MainWindow::application_entries should only contain ApplicationEntry"),
        );
    }

    fn show_application_mime_type_assignment(&self, application_entry: &ApplicationEntry) {
        println!(
            "MainWindow::show_application_mime_type_assignment application_entry: {}",
            application_entry.id()
        );
        let model = NoSelection::new(Some(application_entry.mime_type_assignments()));
        self.imp().application_mime_type_assignment_list_box.bind_model(Some(&model),
            clone!(@weak self as window => @default-panic, move |obj| {
                let model = obj.downcast_ref().expect("The object should be of type `MimeTypeAssignmentEntry`.");
                let row = Self::create_mime_type_assignment_row(model);
                row.upcast()
            }));
    }

    fn create_mime_type_assignment_row(
        mime_type_assignment_entry: &MimeTypeAssignmentEntry,
    ) -> ActionRow {
        let check_button = CheckButton::builder()
            .valign(Align::Center)
            .can_focus(false)
            .build();

        // Create row
        let row = ActionRow::builder()
            .activatable_widget(&check_button)
            .build();
        row.add_prefix(&check_button);

        // Bind properties
        mime_type_assignment_entry
            .bind_property("assigned", &check_button, "active")
            .bidirectional()
            .sync_create()
            .build();
        mime_type_assignment_entry
            .bind_property("id", &row, "title")
            .sync_create()
            .build();

        row
    }

    fn create_application_list_box_row(&self, model: &ApplicationEntry) -> ListBoxRow {
        let label = Label::builder()
            .ellipsize(pango::EllipsizeMode::End)
            .xalign(0.0)
            .build();

        let desktop_entry = &model.desktop_entry();
        label.set_text(desktop_entry.name().unwrap_or("<Unnamed Application>"));

        ListBoxRow::builder().child(&label).build()
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
