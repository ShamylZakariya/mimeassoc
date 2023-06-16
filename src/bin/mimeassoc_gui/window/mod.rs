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
            Ok(components) => {
                // self
                //     .imp()
                //     .components
                //     .set(Rc::new(components))
                //     .expect("Components should only be set once"),

                self.imp()
                    .components
                    .set(Rc::new(RefCell::new(components)))
                    .expect("Components should only be set once");
            }
            Err(e) => panic!("Failed to initialize mimeassoc components, error: {}", e),
        }
    }

    fn components(&self) -> Rc<RefCell<components::Components>> {
        self.imp().components.get().expect("Expect components to have been set").clone()
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
        let components = self.components();
        // let app_db = &components.borrow().app_db;

        // Create row
        let row = ActionRow::builder().build();

        // TODO we need a list model to assign
        let application_drop_down = DropDown::builder().build();
        let applications_model = SingleSelection::new(Some(entry.supported_application_entries()));
        application_drop_down.set_model(Some(&applications_model));

        let list_item_factory = SignalListItemFactory::new();
        list_item_factory.connect_setup(move |_, list_item| {
            let label = Label::new(None);
            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&label));
        });

        list_item_factory.connect_bind(move |_, list_item| {
            let application_entry = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<ApplicationEntry>()
                .expect("The item has to be an `ApplicationEntry`.");

            let label = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<Label>()
                .expect("The child has to be a `Label`.");

            // // We have an ownership issue here... do we need to put components in an Rc?
            // let application_name = application_entry
            //     .get_desktop_entry(app_db)
            //     .expect("Expect desktop_entry_id to be valid")
            //     .name()
            //     .unwrap_or("<Unnamed Application>")
            //     .clone();

            let application_name = "Foo";
            label.set_text(application_name);
        });

        application_drop_down.set_list_factory(Some(&list_item_factory));

        row.add_suffix(&application_drop_down);

        row.set_title(&entry.mime_type());
        row
    }

    /// Called when everything is set up, and ready to show data
    fn ready(&self) {
        let components = self.components();
        let app_db = &components.borrow().app_db;
        let mime_db = &components.borrow().mime_db;

        let mut all_mime_types = mime_db.mime_types();
        all_mime_types.sort();

        let mime_type_entries = all_mime_types
            .iter()
            // Hide any mimetypes for which we have no applications
            .filter(|mt| {
                !app_db
                    .get_desktop_entries_for_mimetype(mt)
                    .is_empty()
            })
            .map(|mt| MimeTypeEntry::new(mt, &components.borrow()))
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
