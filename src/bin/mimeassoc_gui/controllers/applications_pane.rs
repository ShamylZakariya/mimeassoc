use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use adw::{prelude::*, *};
use gtk::{glib::*, *};
use mimeassoc::DesktopEntryId;

use crate::model::*;
use crate::ui::{MainWindow, Strings};

use super::AppController;

mod imp {
    use super::*;
    use std::cell::OnceCell;

    use gtk::glib;

    #[derive(Default)]
    pub struct ApplicationsPaneController {
        pub window: OnceCell<WeakRef<MainWindow>>,
        pub app_controller: OnceCell<WeakRef<AppController>>,
        pub application_entries: OnceCell<gio::ListStore>,
        pub current_selection: RefCell<Option<ApplicationEntry>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ApplicationsPaneController {
        const NAME: &'static str = "ApplicationsPaneController";
        type Type = super::ApplicationsPaneController;
    }

    // Trait shared by all GObjects
    impl ObjectImpl for ApplicationsPaneController {
        fn constructed(&self) {
            Self::parent_constructed(self);
        }
    }
}

glib::wrapper! {
    pub struct ApplicationsPaneController(ObjectSubclass<imp::ApplicationsPaneController>);
}

impl ApplicationsPaneController {
    pub fn new(window: WeakRef<MainWindow>, app_controller: WeakRef<AppController>) -> Self {
        let instance: ApplicationsPaneController = Object::builder().build();
        instance.imp().window.set(window).unwrap();
        instance.imp().app_controller.set(app_controller).unwrap();
        instance.setup();

        instance
    }

    pub fn reload(&self) {
        let application_entry = self.imp().current_selection.borrow().clone();
        if let Some(application_entry) = application_entry {
            self.show_detail(&application_entry);
        }
    }

    pub fn show_application(&self, desktop_entry_id: &DesktopEntryId) {
        let window = self.window();

        let application_entry = ApplicationEntry::new(desktop_entry_id, self.stores());
        self.show_detail(&application_entry);

        // select this app in the list box. This is weirdly complex, perhaps there's a better way?
        let applications_list_box = &window.imp().applications_list_box;
        let application_entries = self.application_entries();
        let count = application_entries.n_items();
        for i in 0..count {
            let model = application_entries.item(i)
                        .expect("Expected a valid row index")
                        .downcast::<ApplicationEntry>()
                        .expect("MainWindow::application_entries() model should contain instances of ApplicationEntry only");

            if let Some(id) = model.desktop_entry_id() {
                if &id == desktop_entry_id {
                    applications_list_box
                        .select_row(applications_list_box.row_at_index(i as i32).as_ref());

                    if i > 0 {
                        crate::ui::scroll_listbox_to_selected_row(applications_list_box.get());
                    }
                }
            }
        }
    }

    fn setup(&self) {
        self.build_model();

        let window = self.window();
        let applications_list_box = &window.imp().applications_list_box;

        // bind the model to the list box
        applications_list_box.bind_model(
            Some(self.application_entries()),
            clone!(@weak self as controller => @default-panic, move |obj| {
                let model = obj
                    .downcast_ref()
                    .unwrap();
                let row = Self::create_application_pane_primary_row(model);
                row.upcast()
            }),
        );

        // Listen for selection
        applications_list_box.connect_row_activated(
            clone!(@weak self as controller => move |_, row|{
                let index = row.index();
                let model = controller.application_entries().item(index as u32)
                    .expect("Expected valid item index")
                    .downcast::<ApplicationEntry>()
                    .expect("MainWindow::application_entries should only contain ApplicationEntry");
                controller.show_detail(&model);
            }),
        );

        // Select first entry
        let row = applications_list_box.row_at_index(0);
        applications_list_box.select_row(row.as_ref());
        self.show_detail(
            &self
                .application_entries()
                .item(0)
                .expect("Expect non-empty application entries model")
                .downcast::<ApplicationEntry>()
                .expect("MainWindow::application_entries should only contain ApplicationEntry"),
        );
    }

    /// Builds the ListStore model which backs the applications listbox
    fn build_model(&self) {
        let stores = self.stores();
        let borrowed_stores = stores.borrow();
        let apps = borrowed_stores.desktop_entry_store();

        let mut all_desktop_entries = apps.desktop_entries();
        all_desktop_entries.sort_by(|a, b| a.cmp_by_name_alpha_inensitive(b));

        let application_entries = all_desktop_entries
            .iter()
            .filter(|de| !de.mime_types().is_empty())
            .map(|de| ApplicationEntry::new(de.id(), stores.clone()))
            .collect::<Vec<_>>();

        let model = gio::ListStore::with_type(ApplicationEntry::static_type());
        model.extend_from_slice(&application_entries);

        self.imp().application_entries.set(model).unwrap();
    }

    fn show_detail(&self, application_entry: &ApplicationEntry) {
        let model = NoSelection::new(Some(application_entry.mime_type_assignments()));

        let window = self.window();
        window.imp().application_to_mime_type_assignment_list_box.bind_model(Some(&model),
            clone!(@weak self as controller, @strong application_entry => @default-panic, move |obj| {
                let model = obj.downcast_ref().expect("The object should be of type `MimeTypeEntry`.");
                let row = controller.create_application_pane_detail_row(&application_entry, model);
                row.upcast()
            }));

        window
            .imp()
            .application_to_mime_type_assignment_list_box
            .set_selection_mode(SelectionMode::None);

        self.imp()
            .current_selection
            .borrow_mut()
            .replace(application_entry.clone());
    }

    fn create_application_pane_primary_row(application_entry: &ApplicationEntry) -> ListBoxRow {
        let application_name_label = Label::builder()
            .wrap(true)
            .wrap_mode(pango::WrapMode::Word)
            .xalign(0.0)
            .css_classes(vec!["display_name"])
            .build();

        let desktop_entry_id_label = Label::builder()
            .wrap(true)
            .wrap_mode(pango::WrapMode::Word)
            .xalign(0.0)
            .css_classes(vec!["desktop_entry_id"])
            .build();

        let content = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(vec!["content"])
            .build();

        content.append(&application_name_label);
        content.append(&desktop_entry_id_label);

        let desktop_entry = &application_entry
            .desktop_entry()
            .expect("Expect to get desktop entry id from ApplicationEntry");
        application_name_label.set_text(desktop_entry.name().unwrap_or("<Unnamed Application>"));
        desktop_entry_id_label.set_text(desktop_entry.id().id());

        ListBoxRow::builder().child(&content).build()
    }

    fn create_application_pane_detail_row(
        self,
        application_entry: &ApplicationEntry,
        mime_type_entry: &MimeTypeEntry,
    ) -> ActionRow {
        let check_button = CheckButton::builder()
            .valign(Align::Center)
            .can_focus(false)
            .build();

        let row = ActionRow::builder()
            .activatable_widget(&check_button)
            .build();
        row.add_prefix(&check_button);

        let mime_type = mime_type_entry.mime_type();
        let desktop_entry = application_entry
            .desktop_entry()
            .expect("Expect to get desktop entry id from ApplicationEntry");
        let (is_system_default_application, is_assigned_application) = {
            let stores = self.stores();
            let stores = stores.borrow();
            let mime_associations_store = stores.mime_associations_store();

            let is_system_default_application = mime_associations_store
                .system_default_application_for(&mime_type)
                == Some(desktop_entry.id());
            let is_assigned_application = mime_associations_store
                .default_application_for(&mime_type)
                == Some(desktop_entry.id());
            (is_system_default_application, is_assigned_application)
        };

        row.set_title(mime_type.to_string().as_str());

        if is_system_default_application {
            row.set_subtitle(
                &Strings::application_is_system_default_handler_for_mimetype_long(
                    &desktop_entry,
                    &mime_type,
                ),
            );

            if is_assigned_application {
                check_button.set_sensitive(false);
                check_button.set_active(true);
                row.set_sensitive(false);
            }
        }

        if is_assigned_application {
            check_button.set_active(true);
        }

        let app_controller = self.app_controller();
        check_button.connect_toggled(clone!(@weak app_controller, @strong desktop_entry, @strong mime_type => move |check_button| {
            if check_button.is_active() {
                app_controller.assign_application_to_mimetype(&mime_type, Some(desktop_entry.id()));
            } else {
                app_controller.assign_application_to_mimetype(&mime_type, None);
            }
        }));

        row
    }

    fn window(&self) -> MainWindow {
        self.imp()
            .window
            .get()
            .unwrap()
            .upgrade()
            .expect("Expect window instance to be valid")
    }

    fn app_controller(&self) -> AppController {
        self.imp()
            .app_controller
            .get()
            .expect("Expect AppController instance to be set")
            .upgrade()
            .expect("Expect AppController instance to be alive")
    }

    fn stores(&self) -> Rc<RefCell<Stores>> {
        self.app_controller().stores()
    }

    fn application_entries(&self) -> &gio::ListStore {
        self.imp().application_entries.get().unwrap()
    }
}
