use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use adw::{prelude::*, *};
use gtk::{glib::*, *};
use mimeassoc::DesktopEntryId;

use crate::model::*;
use crate::resources::Strings;
use crate::ui::MainWindow;

use super::AppController;

mod imp {
    use super::*;
    use std::cell::OnceCell;

    use gtk::glib;

    #[derive(Default)]
    pub struct ApplicationsModeController {
        pub window: OnceCell<WeakRef<MainWindow>>,
        pub app_controller: OnceCell<WeakRef<AppController>>,
        pub application_entries: OnceCell<gio::ListStore>,
        pub current_selection: RefCell<Option<ApplicationEntry>>,
        pub signal_handlers: RefCell<Vec<SignalHandlerId>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for ApplicationsModeController {
        const NAME: &'static str = "ApplicationsModeController";
        type Type = super::ApplicationsModeController;
    }

    // Trait shared by all GObjects
    impl ObjectImpl for ApplicationsModeController {
        fn constructed(&self) {
            Self::parent_constructed(self);
        }
    }
}

glib::wrapper! {
    pub struct ApplicationsModeController(ObjectSubclass<imp::ApplicationsModeController>);
}

impl ApplicationsModeController {
    pub fn new(window: WeakRef<MainWindow>, app_controller: WeakRef<AppController>) -> Self {
        let instance: ApplicationsModeController = Object::builder().build();
        instance.imp().window.set(window).unwrap();
        instance.imp().app_controller.set(app_controller).unwrap();

        instance
    }

    pub fn reload(&self) {
        let application_entry = self.imp().current_selection.borrow().clone();
        if let Some(application_entry) = application_entry {
            self.show_detail(&application_entry);
        }
    }

    pub fn select_application(&self, desktop_entry_id: &DesktopEntryId) {
        let window = self.window();

        let application_entry = ApplicationEntry::new(desktop_entry_id, self.stores());
        self.show_detail(&application_entry);

        // select this app in the list box. This is weirdly complex, perhaps there's a better way?
        let list_box = &window.imp().collections_list;
        let application_entries = self.application_entries();
        let count = application_entries.n_items();
        for i in 0..count {
            let model = application_entries.item(i)
                        .expect("Expected a valid row index")
                        .downcast::<ApplicationEntry>()
                        .expect("ApplicationsModeController::application_entries() model should contain instances of ApplicationEntry only");

            if let Some(id) = model.desktop_entry_id() {
                if &id == desktop_entry_id {
                    list_box.select_row(list_box.row_at_index(i as i32).as_ref());

                    if i > 0 {
                        crate::ui::scroll_listbox_to_selected_row(list_box.get());
                    }
                }
            }
        }
    }

    pub fn on_select_all(&self) {
        self.select_all_or_none(true);
    }

    pub fn on_select_none(&self) {
        self.select_all_or_none(false);
    }

    fn select_all_or_none(&self, all: bool) {
        let app_controller = self.app_controller();
        let application_entry = self.imp().current_selection.borrow().clone();
        if let Some(application_entry) = application_entry {
            let desktop_entry_id = application_entry
                .desktop_entry_id()
                .expect("Expect ApplicationEntry to have a valid DesktopEntryId");

            let mime_types = application_entry.mime_type_assignments();
            for i in 0..mime_types.n_items() {
                let mime_type_entry = mime_types.item(i).expect("Expected a valid row index")
                        .downcast::<MimeTypeEntry>()
                        .expect("ApplicationsModeController::application_entries() model should contain instances of MimeTypeEntry only");

                if all {
                    app_controller.assign_application_to_mimetype(
                        &mime_type_entry.mime_type(),
                        Some(&desktop_entry_id),
                    );
                } else {
                    app_controller
                        .assign_application_to_mimetype(&mime_type_entry.mime_type(), None);
                }
            }

            self.show_detail(&application_entry);
        }
    }

    pub fn activate(&self) {
        self.build_model();

        let window = self.window();
        let list_box = &window.imp().collections_list;

        // reveal or hide components for applications mode
        window.imp().select_all_none_buttons.set_visible(true);
        window
            .imp()
            .mime_type_mode_detail_info_label
            .set_visible(false);

        // bind the model to the list box
        list_box.bind_model(
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
        let sid = list_box.connect_row_activated(
            clone!(@weak self as controller => move |_, row|{
                let index = row.index();
                let model = controller.application_entries().item(index as u32)
                    .expect("Expected valid item index")
                    .downcast::<ApplicationEntry>()
                    .expect("ApplicationsModeController::application_entries should only contain ApplicationEntry");
                controller.show_detail(&model);
            }),
        );

        // record our signal handlers so we can clean up later
        let signal_handler_ids = vec![sid];
        self.imp().signal_handlers.replace(signal_handler_ids);

        // If an item was previously selected, re-select it. Otherwise select the first item
        let current_selection = self.imp().current_selection.borrow().clone();
        if let Some(current_selection) = current_selection.and_then(|s| s.desktop_entry_id()) {
            self.select_application(&current_selection);
        } else {
            let model = self.application_entries();
            if let Some(first_item) = model
                .item(0)
                .and_downcast::<ApplicationEntry>()
                .and_then(|a| a.desktop_entry_id())
            {
                self.select_application(&first_item);
            }
        }
    }

    pub fn deactivate(&self) {
        let window = self.window();
        let signal_handler_ids = self.imp().signal_handlers.take();
        for s_id in signal_handler_ids.into_iter() {
            window.imp().collections_list.disconnect(s_id);
        }
    }

    /// Builds the ListStore model which backs the applications listbox
    fn build_model(&self) {
        if self.imp().application_entries.get().is_some() {
            return;
        }

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
        let mime_type_assignments = application_entry.mime_type_assignments();
        let model = NoSelection::new(Some(mime_type_assignments));

        let window = self.window();
        let list_box = &window.imp().detail_list;

        list_box.bind_model(Some(&model),
            clone!(@weak self as controller, @strong application_entry => @default-panic, move |obj| {
                let model = obj.downcast_ref().expect("The object should be of type `MimeTypeEntry`.");
                let row = controller.create_application_pane_detail_row(&application_entry, model);
                row.upcast()
            }));

        list_box.set_selection_mode(SelectionMode::None);

        self.imp()
            .current_selection
            .borrow_mut()
            .replace(application_entry.clone());

        self.update_detail_labels(application_entry);
        self.update_select_all_and_none_buttons();
    }

    fn update_detail_labels(&self, application_entry: &ApplicationEntry) {
        let desktop_entry = &application_entry
            .desktop_entry()
            .expect("Expect to get desktop entry id from ApplicationEntry");

        let window = self.window();
        let detail_label_primary = &window.imp().detail_title;
        let detail_label_secondary = &window.imp().detail_sub_title;

        detail_label_primary.set_text(desktop_entry.name().unwrap_or("<Unnamed Application>"));
        detail_label_secondary.set_text(desktop_entry.id().id());
    }

    fn update_select_all_and_none_buttons(&self) {
        let application_entry = self.imp().current_selection.borrow().clone();
        if let Some(application_entry) = application_entry {
            let mime_type_entries = application_entry.mime_type_assignments();

            let (can_select_all, can_select_none) = if mime_type_entries.n_items() > 1 {
                let desktop_entry_id = application_entry
                    .desktop_entry_id()
                    .expect("Expect ApplicationEntry to have a DesktopEntryId");
                let n_items = mime_type_entries.n_items();
                let stores = self.stores();
                let stores = stores.borrow();
                let mime_associations_store = stores.mime_associations_store();
                let mut num_assigned = 0;

                for i in 0..n_items {
                    let mime_type_entry = mime_type_entries.item(i).expect("Expected a valid row index")
                            .downcast::<MimeTypeEntry>()
                            .expect("ApplicationsModeController::application_entries() model should contain instances of MimeTypeEntry only");

                    let is_assigned_application = mime_associations_store
                        .default_application_for(&mime_type_entry.mime_type())
                        == Some(&desktop_entry_id);
                    if is_assigned_application {
                        num_assigned += 1;
                    }
                }

                (num_assigned < n_items, num_assigned > 0)
            } else {
                (false, false)
            };

            let window = self.window();
            window.imp().select_all_button.set_sensitive(can_select_all);
            window
                .imp()
                .select_none_button
                .set_sensitive(can_select_none);
        }
    }

    fn create_application_pane_primary_row(application_entry: &ApplicationEntry) -> ListBoxRow {
        let application_name_label = Label::builder()
            .wrap(true)
            .wrap_mode(pango::WrapMode::Word)
            .ellipsize(pango::EllipsizeMode::Middle)
            .xalign(0.0)
            .css_classes(vec!["display_name"])
            .build();

        let desktop_entry_id_label = Label::builder()
            .wrap(true)
            .wrap_mode(pango::WrapMode::Word)
            .ellipsize(pango::EllipsizeMode::Middle)
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
        check_button.connect_toggled(clone!(@weak self as controller, @weak app_controller, @strong desktop_entry, @strong mime_type => move |check_button| {
            if check_button.is_active() {
                app_controller.assign_application_to_mimetype(&mime_type, Some(desktop_entry.id()));
            } else {
                app_controller.assign_application_to_mimetype(&mime_type, None);
            }
            controller.update_select_all_and_none_buttons();
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
