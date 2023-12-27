use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use adw::{prelude::*, *};
use gtk::{glib::*, *};
use mimeassoc::*;

use crate::model::*;
use crate::resources::Strings;
use crate::ui::MainWindow;

use super::AppController;

mod imp {
    use super::*;
    use std::cell::OnceCell;

    use gtk::glib;

    #[derive(Default)]
    pub struct MimeTypesModeController {
        pub window: OnceCell<WeakRef<MainWindow>>,
        pub app_controller: OnceCell<WeakRef<AppController>>,
        pub mime_type_entries: OnceCell<gio::ListStore>,

        pub application_check_button_group: RefCell<Option<CheckButton>>,
        pub current_selection: RefCell<Option<MimeTypeEntry>>,
        pub signal_handlers: RefCell<Vec<SignalHandlerId>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for MimeTypesModeController {
        const NAME: &'static str = "MimeTypesModeController";
        type Type = super::MimeTypesModeController;
    }

    // Trait shared by all GObjects
    impl ObjectImpl for MimeTypesModeController {
        fn constructed(&self) {
            Self::parent_constructed(self);
        }
    }
}

glib::wrapper! {
    pub struct MimeTypesModeController(ObjectSubclass<imp::MimeTypesModeController>);
}

impl MimeTypesModeController {
    pub fn new(window: WeakRef<MainWindow>, app_controller: WeakRef<AppController>) -> Self {
        let instance: MimeTypesModeController = Object::builder().build();
        instance.imp().window.set(window).unwrap();
        instance.imp().app_controller.set(app_controller).unwrap();
        instance
    }

    pub fn reload(&self) {
        let mime_type_entry = self.imp().current_selection.borrow().clone();
        if let Some(mime_type_entry) = mime_type_entry {
            self.show_detail(&mime_type_entry);
        }
    }

    pub fn mime_type_entries(&self) -> &gio::ListStore {
        self.imp().mime_type_entries.get().unwrap()
    }

    pub fn select_mime_type(&self, mime_type: &MimeType) {
        let window = self.window();

        // Show the detail pane for this mime type
        let mime_type_entry = MimeTypeEntry::new(mime_type, self.stores());
        self.show_detail(&mime_type_entry);

        // Select this mime type in the list box
        let list_box = &window.imp().collections_list;
        let mime_type_entries = self.mime_type_entries();
        let count = mime_type_entries.n_items();
        for i in 0..count {
            let model = mime_type_entries.item(i)
                        .expect("Expected a valid row index")
                        .downcast::<MimeTypeEntry>()
                        .expect("MimeTypesModeController::mime_type_entries() model should contain instances of MimeTypeEntry only");
            if &model.mime_type() == mime_type {
                if let Some(row) = list_box.row_at_index(i as i32) {
                    list_box.select_row(Some(&row));

                    if i > 0 {
                        crate::ui::scroll_listbox_to_selected_row(list_box.get());
                    }
                }

                break;
            }
        }
    }

    pub fn activate(&self) {
        self.build_model();

        // bind to selection events
        let window = self.window();
        let list_box = &window.imp().collections_list;

        list_box.bind_model(
            Some(self.mime_type_entries()),
            clone!(@weak self as controller => @default-panic, move | obj | {
                let model = obj.downcast_ref().unwrap();
                let row = controller.create_selection_row(model);
                row.upcast()
            }),
        );

        let s_id = list_box.connect_row_activated(clone!(@weak self as controller => move |_, row|{
            let index = row.index();
            let model = controller.mime_type_entries().item(index as u32)
                .expect("Expected a valid row index")
                .downcast::<MimeTypeEntry>()
                .expect("MimeTypesModeController::mime_type_entries() model should contain instances of MimeTypeEntry only");
            controller.show_detail(&model);
        }));

        self.imp().signal_handlers.replace(vec![s_id]);

        // Select first entry
        let row = list_box.row_at_index(0);
        list_box.select_row(row.as_ref());
        self.show_detail(
            &self
                .mime_type_entries()
                .item(0)
                .expect("Expect non-empty mime type entries model")
                .downcast::<MimeTypeEntry>()
                .expect(
                    "MimeTypesModeController::mime_type_entries() model should contain instances of MimeTypeEntry only",
                ));
    }

    pub fn deactivate(&self) {
        let window = self.window();
        let signal_handler_ids = self.imp().signal_handlers.take();
        for s_id in signal_handler_ids.into_iter() {
            window.imp().collections_list.disconnect(s_id);
        }
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

    /// Builds the ListStore model which backs the mime types listbox
    fn build_model(&self) {
        if self.imp().mime_type_entries.get().is_some() {
            return;
        }

        let stores = self.stores();
        let borrowed_stores = stores.borrow();
        let mime_associations_store = borrowed_stores.mime_associations_store();

        let mut all_mime_types = mime_associations_store.mime_types();
        all_mime_types.sort();

        let mime_type_entries = all_mime_types
            .iter()
            .map(|mt| MimeTypeEntry::new(mt, stores.clone()))
            .filter(|e| e.supported_application_entries().n_items() > 0)
            .collect::<Vec<_>>();

        let model = gio::ListStore::with_type(MimeTypeEntry::static_type());
        model.extend_from_slice(&mime_type_entries);

        self.imp().mime_type_entries.set(model).unwrap();
    }

    /// Creates a row for the primary/selection list box
    fn create_selection_row(&self, model: &MimeTypeEntry) -> ListBoxRow {
        let stores = self.stores();
        let stores = stores.borrow();
        let mime_info_store = stores.mime_info_store();

        let mime_type = &model.mime_type();
        let mime_info = mime_info_store.get_info_for_mime_type(mime_type);

        let mime_type_label = Label::builder()
            .wrap(true)
            .wrap_mode(pango::WrapMode::Word)
            .xalign(0.0)
            .css_classes(vec!["mime-type"])
            .build();

        mime_type_label.set_text(&mime_type.to_string());

        let content = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .css_classes(vec!["content"])
            .build();

        content.append(&mime_type_label);

        if let Some(name) = mime_info.and_then(|info| info.comment()) {
            let file_type_name_label = Label::builder()
                .wrap(true)
                .wrap_mode(pango::WrapMode::Word)
                .xalign(0.0)
                .css_classes(vec!["mime-type-description"])
                .build();

            file_type_name_label.set_text(name);

            content.append(&file_type_name_label);
        }

        ListBoxRow::builder().child(&content).build()
    }

    /// Shows detail for the provided MimeTypeEntry - this is generally
    /// called in response to user tapping a selection in the primary list box
    fn show_detail(&self, mime_type_entry: &MimeTypeEntry) {
        log::warn!("show_detail unimplemented")

        // // flag that we're currently viewing this mime type
        // self.imp()
        //     .current_selection
        //     .borrow_mut()
        //     .replace(mime_type_entry.clone());

        // let window = self.window();
        // let list_box = &window.imp().mime_type_pane_detail_applications_list_box;
        // list_box.set_selection_mode(SelectionMode::None);

        // // Reset the application check button group before building the list; it will be
        // // assigned to the first created list item, and if there are subsequent items, they
        // // will use it as a group, making them into radio buttons.
        // self.imp()
        //     .application_check_button_group
        //     .borrow_mut()
        //     .take();

        // let application_entries = mime_type_entry.supported_application_entries();
        // let model_count = application_entries.n_items();

        // //insert an empty entry at beginning of list - this will be the None entry
        // application_entries.insert(0, &ApplicationEntry::none());

        // let model = NoSelection::new(Some(application_entries));
        // list_box.bind_model(Some(&model),
        //     clone!(@weak self as controller, @strong mime_type_entry => @default-panic, move |obj| {
        //         let application_entry = obj.downcast_ref().expect("The object should be of type `ApplicationEntry`.");
        //         controller.create_detail_row(&mime_type_entry, application_entry, model_count).upcast()
        //     }));

        // // Update the info label - basically, if only one application is shown, and it is the
        // // system default handler for the mime type, it will be presented in a disabled state
        // // in ::create_application_row, and here we show an info label to explain why

        // let info_label = &window.imp().mime_type_pane_detail_info_label;
        // let show_info_label = if model_count == 1 {
        //     // if the number of items is 1, and that item is the system default, show the info message
        //     let desktop_entry = model
        //         .item(1) // Recall, there's an empty ApplicationEntry at position 0
        //         .unwrap()
        //         .downcast_ref::<ApplicationEntry>()
        //         .unwrap()
        //         .desktop_entry()
        //         .expect("Expect to receive a DesktopEntry from the ApplicationEntry");

        //     let mime_type = mime_type_entry.mime_type();

        //     let stores = self.stores();
        //     let stores = stores.borrow();
        //     let mime_association_store = stores.mime_associations_store();
        //     let is_system_default = mime_association_store
        //         .system_default_application_for(&mime_type)
        //         == Some(desktop_entry.id());

        //     if is_system_default {
        //         // TODO: Move this into some kind of string table
        //         let info_message =
        //             Strings::single_default_application_info_message(&desktop_entry, &mime_type);
        //         info_label.set_label(&info_message);
        //         true
        //     } else {
        //         false
        //     }
        // } else {
        //     false
        // };

        // info_label.set_visible(show_info_label);
    }

    fn create_detail_row(
        &self,
        mime_type_entry: &MimeTypeEntry,
        application_entry: &ApplicationEntry,
        num_application_entries_in_list: u32,
    ) -> ActionRow {
        let mime_type = mime_type_entry.mime_type();
        let check_button = CheckButton::builder()
            .valign(Align::Center)
            .can_focus(false)
            .build();
        let app_controller = self.app_controller();

        let row = if application_entry.desktop_entry_id().is_some() {
            let (is_system_default_application, is_assigned_application) = {
                let stores = self.stores();
                let stores = stores.borrow();
                let mime_associations_store = stores.mime_associations_store();
                let desktop_entry_id = application_entry
                    .desktop_entry_id()
                    .expect("Expect to get desktop entry id");

                let is_default_application = mime_associations_store
                    .system_default_application_for(&mime_type)
                    == Some(&desktop_entry_id);
                let is_assigned_application = mime_associations_store
                    .default_application_for(&mime_type)
                    == Some(&desktop_entry_id);
                (is_default_application, is_assigned_application)
            };

            check_button.set_active(is_assigned_application);

            check_button.connect_toggled(
                clone!(@weak self as controller, @weak app_controller, @strong application_entry, @strong mime_type => move |check_button| {
                    let is_single_checkbox = num_application_entries_in_list == 1;

                    if check_button.is_active() {
                        let desktop_entry_id = application_entry.desktop_entry_id().expect("Expect to get desktop entry id");
                        app_controller.assign_application_to_mimetype(&mime_type, Some(&desktop_entry_id));
                    } else if is_single_checkbox {
                        // only send the unchecked signal if this is a single checkbox, not a multi-element radio button
                        app_controller.assign_application_to_mimetype(&mime_type, None);
                    }
                }),
            );
            let row = ActionRow::builder()
                .activatable_widget(&check_button)
                .build();
            row.add_prefix(&check_button);

            let desktop_entry = application_entry
                .desktop_entry()
                .expect("Expect to get desktop entry id from ApplicationEntry");
            let title = desktop_entry.name().unwrap_or("<Unnamed Application>");
            row.set_title(title);

            if is_system_default_application {
                row.set_subtitle(
                    Strings::application_is_system_default_handler_for_mimetype_short(&mime_type)
                        .as_str(),
                );
            }

            row
        } else {
            // This is a "None" row; the check button will be selected if nothing is assigned to this mime type.
            // Selecting this item will unset bindings to this mimetype

            let stores = self.stores();
            let stores = stores.borrow();
            let mime_associations_store = stores.mime_associations_store();
            let desktop_entry_store = stores.desktop_entry_store();

            let an_application_is_assigned = mime_associations_store
                .default_application_for(&mime_type)
                .is_some_and(|desktop_entry_id| {
                    desktop_entry_store
                        .find_desktop_entry_with_id(desktop_entry_id)
                        .is_some_and(|e| e.appears_valid_application())
                });

            let can_assign_none = !mime_associations_store
                .system_default_application_for(&mime_type)
                .is_some_and(|desktop_entry_id| {
                    desktop_entry_store
                        .find_desktop_entry_with_id(desktop_entry_id)
                        .is_some_and(|e| e.appears_valid_application())
                });

            check_button.set_active(!an_application_is_assigned);
            check_button.connect_toggled(
                clone!(@weak self as controller, @strong mime_type => move |check_button| {
                    if check_button.is_active() {
                        controller.on_select_none(&mime_type);
                    }
                }),
            );

            let row = ActionRow::builder()
                .activatable_widget(&check_button)
                .build();
            row.add_prefix(&check_button);
            row.set_title(Strings::assign_no_application_list_item());
            row.set_sensitive(can_assign_none);

            row
        };

        // RadioButtons work by putting check buttons in a group; we check if the group exists
        // and add this check button if it does; otherwise, we need to make a new group from
        // our first check button. NOTE: We early-return here because we are borrowing the
        // application_check_button_group, and need to let it go out of scope to borrow_mut later.
        if let Some(group) = self.imp().application_check_button_group.borrow().as_ref() {
            check_button.set_group(Some(group));
            return row;
        }

        // this is the first vended row, use its check button to stand as the group
        self.imp()
            .application_check_button_group
            .borrow_mut()
            .replace(check_button);

        row
    }

    fn on_select_none(&self, mime_type: &MimeType) {
        let application_controller = self.app_controller();
        application_controller.assign_application_to_mimetype(mime_type, None);
        application_controller.reload_active_page();
    }
}
