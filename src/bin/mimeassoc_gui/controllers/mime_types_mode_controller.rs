use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use adw::{prelude::*, *};
use gtk::{glib::*, *};
use mimeassoc::*;

use crate::model::*;
use crate::resources::Strings;
use crate::ui::MainWindow;

use super::app_controller::{DetailViewMode, FilterPrecisionChange};
use super::AppController;

mod imp {
    use super::*;
    use std::cell::OnceCell;

    use gtk::glib;

    #[derive(Default)]
    pub struct MimeTypesModeController {
        pub window: OnceCell<WeakRef<MainWindow>>,
        pub app_controller: OnceCell<WeakRef<AppController>>,
        pub application_check_button_group: RefCell<Option<CheckButton>>,
        pub current_selection: RefCell<Option<MimeTypeEntry>>,
        pub signal_handlers: RefCell<Vec<SignalHandlerId>>,
        pub current_search_string: RefCell<Option<String>>,
        pub filter_model: OnceCell<FilterListModel>,
        pub selection_model: OnceCell<SingleSelection>,
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

    pub fn reload_detail(&self) {
        if let Some(mime_type_entry) = self.current_selection() {
            self.show_detail(&mime_type_entry);
        }
    }

    pub fn select_mime_type(&self, mime_type: &MimeType) {
        self.app_controller()
            .set_detail_view_mode(DetailViewMode::ShowDetail);

        let window = self.window();
        let list_box = &window.imp().collections_list;

        crate::ui::select_listbox_row_and_scroll_to_visible(
            list_box.get(),
            self.collections_list_model(),
            |entry: MimeTypeEntry| &entry.mime_type() == mime_type,
        );

        let mime_type_entry = MimeTypeEntry::new(mime_type, self.stores());

        // Show the detail pane for this mime type
        self.show_detail(&mime_type_entry);

        // Record current selection
        self.imp()
            .current_selection
            .borrow_mut()
            .replace(mime_type_entry);
    }

    /// Called by AppController to make this mode controller "active"; takes ownership of
    /// the primary and detail views, etc.
    pub fn activate(&self) {
        self.build_model();

        let window = self.window();
        let list_box = &window.imp().collections_list;

        // hide the select all/none buttons in the footer
        window.imp().select_all_none_buttons.set_visible(false);

        // bind the model to the list box
        list_box.bind_model(
            Some(self.collections_list_model()),
            clone!(@weak self as controller => @default-panic, move | obj | {
                let mime_type_entry = obj.downcast_ref().unwrap();
                let row = controller.create_primary_row(mime_type_entry);
                row.upcast()
            }),
        );

        // bind to selection events
        let s_id = list_box.connect_row_selected(clone!(@weak self as controller => move |_, row|{
            if let Some(row) = row {
                let index = row.index();
                let mime_type_entry = controller.collections_list_model().item(index as u32)
                    .and_downcast::<MimeTypeEntry>()
                    .expect("MimeTypesModeController::collections_list_model() model should contain instances of MimeTypeEntry only");
                controller.show_detail(&mime_type_entry);
            }
        }));

        self.imp().signal_handlers.replace(vec![s_id]);

        // Apply the current search string
        let current_search_string = self.app_controller().current_search_string();
        self.on_search_changed(
            current_search_string.as_deref(),
            FilterPrecisionChange::MorePrecise,
        );

        if current_search_string.is_none() {
            self.apply_current_selection();
        }
    }

    /// Called by the AppController to notify that this mode controller no longer has ownership of the UI.
    pub fn deactivate(&self) {
        let window = self.window();

        let signal_handler_ids = self.imp().signal_handlers.take();
        for s_id in signal_handler_ids.into_iter() {
            window.imp().collections_list.disconnect(s_id);
        }
    }

    pub fn on_search_changed(
        &self,
        new_search_string: Option<&str>,
        precision_change: FilterPrecisionChange,
    ) {
        self.imp()
            .current_search_string
            .replace(new_search_string.map(str::to_string));

        let filter_model = self.filter_model();
        filter_model
            .filter()
            .unwrap()
            .changed(FilterChange::Different);

        match precision_change {
            FilterPrecisionChange::None | FilterPrecisionChange::MorePrecise => {
                // select the first element in the model, but only if the search string is non-empty
                if new_search_string.is_some() {
                    if let Some(first_element) =
                        filter_model.item(0).and_downcast_ref::<MimeTypeEntry>()
                    {
                        self.select_mime_type(&first_element.mime_type());
                    } else {
                        self.app_controller()
                            .set_detail_view_mode(DetailViewMode::ShowNoResultsFound);
                    }
                }
            }
            FilterPrecisionChange::LessPrecise => {
                // re-select the current selection in the collections list; we need to do
                // this since the selection state is lost (?) when filtering is updated.
                self.apply_current_selection();
            }
        }
    }

    fn filter_func(&self, mime_type_entry: &MimeTypeEntry) -> bool {
        if let Some(current_search_string) = self.imp().current_search_string.borrow().as_ref() {
            let current_search_string = current_search_string.to_lowercase();

            let stores = self.stores();
            let stores = stores.borrow();
            let mime_info_store = stores.mime_info_store();
            let mime_type = &mime_type_entry.mime_type();

            if mime_type
                .to_string()
                .to_lowercase()
                .contains(current_search_string.as_str())
            {
                return true;
            }

            let mime_info = mime_info_store.get_info_for_mime_type(mime_type);

            if let Some(comment) = mime_info.and_then(|info| info.comment()) {
                if comment
                    .to_lowercase()
                    .contains(current_search_string.as_str())
                {
                    return true;
                }
            }

            false
        } else {
            true
        }
    }

    /// Builds the ListStore model which backs the mime types listbox
    fn build_model(&self) {
        if self.imp().filter_model.get().is_some() {
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

        let list_store = gio::ListStore::with_type(MimeTypeEntry::static_type());
        list_store.extend_from_slice(&mime_type_entries);

        let filter = CustomFilter::new(
            clone!(@weak self as controller => @default-panic, move |obj|{
                    let mime_type_entry = obj
                        .downcast_ref()
                        .unwrap();
                controller.filter_func(mime_type_entry)
            }),
        );

        let filter_model = FilterListModel::new(Some(list_store.clone()), Some(filter));

        self.imp().filter_model.set(filter_model.clone()).unwrap();

        let selection_model = SingleSelection::new(Some(filter_model));
        selection_model.set_autoselect(true);
        selection_model.set_can_unselect(false);
        selection_model.select_item(0, true);
        self.imp().selection_model.set(selection_model).unwrap();
    }

    /// Creates a row for the primary/selection list box
    fn create_primary_row(&self, model: &MimeTypeEntry) -> ListBoxRow {
        let stores = self.stores();
        let stores = stores.borrow();
        let mime_info_store = stores.mime_info_store();

        let mime_type = &model.mime_type();
        let mime_info = mime_info_store.get_info_for_mime_type(mime_type);

        let mime_type_label = Label::builder()
            .wrap(true)
            .wrap_mode(pango::WrapMode::Word)
            .ellipsize(pango::EllipsizeMode::Middle)
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
                .ellipsize(pango::EllipsizeMode::Middle)
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
        self.update_detail_labels(mime_type_entry);

        // flag that we're currently viewing this mime type
        self.imp()
            .current_selection
            .borrow_mut()
            .replace(mime_type_entry.clone());

        let window = self.window();
        let list_box = &window.imp().detail_list;
        list_box.set_selection_mode(SelectionMode::None);

        // Reset the application check button group before building the list; it will be
        // assigned to the first created list item, and if there are subsequent items, they
        // will use it as a group, making them into radio buttons.
        self.imp()
            .application_check_button_group
            .borrow_mut()
            .take();

        let application_entries = mime_type_entry.supported_application_entries();
        let num_application_entries = application_entries.n_items();

        //insert an empty entry at beginning of list - this will be the None entry
        application_entries.insert(0, &ApplicationEntry::none());

        let model = NoSelection::new(Some(application_entries));
        list_box.bind_model(Some(&model),
            clone!(@weak self as controller, @strong mime_type_entry => @default-panic, move |obj| {
                let application_entry = obj.downcast_ref().expect("The object should be of type `ApplicationEntry`.");
                controller.create_detail_row(&mime_type_entry, application_entry, num_application_entries).upcast()
            }));

        // Update the info label - basically, if only one application is shown, and it is the
        // system default handler for the mime type, it will be presented in a disabled state
        // in ::create_application_row, and here we show an info label to explain why

        let info_label = &window.imp().mime_type_mode_detail_info_label;
        let show_info_label = if num_application_entries == 1 {
            // if the number of items is 1, and that item is the system default, show the info message
            let desktop_entry = model
                .item(1) // Recall, there's an empty ApplicationEntry at position 0
                .unwrap()
                .downcast_ref::<ApplicationEntry>()
                .unwrap()
                .desktop_entry()
                .expect("Expect to receive a DesktopEntry from the ApplicationEntry");

            let mime_type = mime_type_entry.mime_type();

            let stores = self.stores();
            let stores = stores.borrow();
            let mime_association_store = stores.mime_associations_store();
            let is_system_default = mime_association_store
                .system_default_application_for(&mime_type)
                == Some(desktop_entry.id());

            if is_system_default {
                // TODO: Move this into some kind of string table
                let info_message =
                    Strings::single_default_application_info_message(&desktop_entry, &mime_type);
                info_label.set_label(&info_message);
                true
            } else {
                false
            }
        } else {
            false
        };

        info_label.set_visible(show_info_label);
    }

    fn update_detail_labels(&self, mime_type_entry: &MimeTypeEntry) {
        let stores = self.stores();
        let stores = stores.borrow();
        let mime_info_store = stores.mime_info_store();

        let mime_type = &mime_type_entry.mime_type();
        let mime_info = mime_info_store.get_info_for_mime_type(mime_type);

        let window = self.window();
        let detail_label_primary = &window.imp().detail_title;
        let detail_label_secondary = &window.imp().detail_sub_title;

        detail_label_primary.set_text(&mime_type_entry.id());

        if let Some(name) = mime_info.and_then(|info| info.comment()) {
            detail_label_secondary.set_text(name);
        } else {
            detail_label_secondary.set_text("");
        }
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
        application_controller.reload_active_mode();
    }

    /// Cause the UI to select the current selection and show its detail
    fn apply_current_selection(&self) {
        if let Some(current_selection) = self.current_selection() {
            // If an item was previously selected, re-select it. Otherwise select the first item
            self.select_mime_type(&current_selection.mime_type());
        } else {
            let list_store = self.collections_list_model();
            if let Some(first_item) = list_store.item(0).and_downcast::<MimeTypeEntry>() {
                self.select_mime_type(&first_item.mime_type());
            }
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

    fn filter_model(&self) -> &FilterListModel {
        self.imp().filter_model.get().unwrap()
    }

    fn collections_list_model(&self) -> &SingleSelection {
        self.imp().selection_model.get().unwrap()
    }

    fn current_selection(&self) -> Option<MimeTypeEntry> {
        self.imp().current_selection.borrow().clone()
    }
}
