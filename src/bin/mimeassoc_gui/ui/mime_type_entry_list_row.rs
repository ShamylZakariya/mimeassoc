use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Object;
use gtk::glib::{self, clone, *};
use mimeassoc::*;

use crate::model::*;

mod imp {
    use super::*;
    use std::cell::RefCell;

    use gtk::glib::SignalHandlerId;
    use gtk::traits::{BoxExt, WidgetExt};
    use gtk::{glib, *};

    // Object holding the state
    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/org/zakariya/MimeAssoc/mime_type_entry_list_row.ui")]
    pub struct MimeTypeEntryListRow {
        pub on_selection_changed:
            RefCell<Option<std::boxed::Box<dyn Fn(&DesktopEntryId, &MimeType)>>>,
        pub combobox_changed_handler_id: RefCell<Option<SignalHandlerId>>,

        // UI Bindings
        #[template_child]
        pub applications_combo_box: TemplateChild<ComboBoxText>,

        #[template_child]
        pub label_box: TemplateChild<Box>,

        #[template_child]
        pub content_label: TemplateChild<Label>,

        #[template_child]
        pub info_label: TemplateChild<Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MimeTypeEntryListRow {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "MimeTypeEntryListRow";
        type Type = super::MimeTypeEntryListRow;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for MimeTypeEntryListRow {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().set_spacing(12);
            self.label_box.set_hexpand(true);
            self.content_label.set_halign(Align::Start);
            // self.content_label.set_hexpand(true);
            self.info_label.set_halign(Align::Start);
            self.info_label.set_opacity(0.7);
            // self.info_label.set_hexpand(true);
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for MimeTypeEntryListRow {}

    // Trait shared by all boxes
    impl BoxImpl for MimeTypeEntryListRow {}
}

glib::wrapper! {
    pub struct MimeTypeEntryListRow(ObjectSubclass<imp::MimeTypeEntryListRow>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for MimeTypeEntryListRow {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl MimeTypeEntryListRow {
    pub fn new(on_application_selected: impl Fn(&DesktopEntryId, &MimeType) + 'static) -> Self {
        let obj: MimeTypeEntryListRow = Object::builder().build();
        obj.imp()
            .on_selection_changed
            .set(Some(Box::new(on_application_selected)));
        obj
    }

    pub fn bind(&self, mime_type_entry: &MimeTypeEntry, stores: Rc<RefCell<Stores>>) {
        self.set_spacing(12);

        self.bind_labels(mime_type_entry, stores.clone());
        self.bind_default_handler_combobox(mime_type_entry, stores);
    }

    pub fn unbind(&self) {
        let applications_combobox = self.imp().applications_combo_box.get();
        applications_combobox.remove_all();

        if let Some(bar) = self.imp().combobox_changed_handler_id.take() {
            applications_combobox.disconnect(bar);
        }
    }

    fn bind_labels(&self, mime_type_entry: &MimeTypeEntry, stores: Rc<RefCell<Stores>>) {
        let mime_type = &mime_type_entry.mime_type();
        let content_label = self.imp().content_label.get();
        content_label.set_text(&mime_type.to_string());

        let info_label = self.imp().info_label.get();
        let mime_info_store = &stores.borrow().mime_info_store;
        let info_label_text: Option<String> =
            if let Some(mime_info) = mime_info_store.get_info_for_mime_type(mime_type) {
                info_label.set_visible(true);
                let extensions = mime_info.extensions().join(", ").to_uppercase();
                if !extensions.is_empty() {
                    Some(format!(
                        "{}: {}",
                        mime_info.comment().unwrap_or(""),
                        extensions,
                    ))
                } else {
                    Some(mime_info.comment().unwrap_or("").to_string())
                }
            } else {
                info_label.set_visible(false);
                None
            };

        if let Some(info_label_text) = info_label_text {
            info_label.set_text(&info_label_text);
        }
    }

    fn bind_default_handler_combobox(
        &self,
        mime_type_entry: &MimeTypeEntry,
        stores: Rc<RefCell<Stores>>,
    ) {
        let applications_combobox = self.imp().applications_combo_box.get();
        let mime_type = &mime_type_entry.mime_type();
        let applications = mime_type_entry.supported_application_entries();
        let mut assigned_id: Option<String> = None;

        // Populate application combobox
        for i in 0_u32..applications.n_items() {
            if let Some(entry) = applications.item(i) {
                let entry = entry
                    .downcast_ref::<ApplicationEntry>()
                    .expect("Expect applications list store to contain ApplicationEntry");
                let desktop_entry_id = entry.desktop_entry_id();
                let id = desktop_entry_id.to_string();
                let display_name = entry.desktop_entry().name().unwrap_or(&id).to_string();

                let is_assigned = if let Some(assigned_desktop_entry) = stores
                    .borrow()
                    .mime_associations_store
                    .assigned_application_for(mime_type)
                {
                    assigned_desktop_entry == &desktop_entry_id
                } else {
                    false
                };

                applications_combobox.append(Some(&id), &display_name);

                if is_assigned {
                    applications_combobox.set_active_id(Some(&id));
                    assigned_id = Some(id);
                }
            }
        }

        // Set application combobox behavior based on whether there are options available to user
        match applications.n_items() {
            0 => {
                unreachable!("Expected that unhandlable mimetype are hidden from the list");
            }
            1 => {
                // if only one application can handle the mimetype, and that application
                // is already assigned; disable the combobox
                let enabled: bool = if let Some(active_id) = applications_combobox.active_id() {
                    let active_id = active_id.to_string();
                    Some(active_id) != assigned_id
                } else {
                    true
                };

                applications_combobox.set_sensitive(enabled);
            }
            _ => {
                // multiple applications support this mime type, the combobox can be enabled
                applications_combobox.set_sensitive(true);
            }
        }

        // listen to `changed` event on combobox
        let callback_id = applications_combobox.connect_changed(clone!(@weak mime_type_entry, @weak stores, @weak self as row => move |a|{
            if let Some(active_id) = a.active_id() {
                let active_id = active_id.as_str();
                let mime_type = mime_type_entry.mime_type();
                let desktop_entry_id = DesktopEntryId::parse(active_id).expect("Expected valid desktop entry id");
                row.on_application_selected(&desktop_entry_id, &mime_type);
            }
        }));

        self.imp()
            .combobox_changed_handler_id
            .replace(Some(callback_id));
    }

    fn on_application_selected(&self, desktop_entry_id: &DesktopEntryId, mime_type: &MimeType) {
        if let Some(callback) = self.imp().on_selection_changed.borrow().as_ref() {
            callback(desktop_entry_id, mime_type);
        }
    }
}
