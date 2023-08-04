use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use adw::subclass::prelude::*;
use adw::{prelude::*, *};
use glib::subclass::*;
use gtk::{glib::*, *};
use mimeassoc::*;

use crate::model::*;
use crate::ui::MimeTypeEntryListRow;

mod imp {
    use super::*;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/zakariya/MimeAssoc/main_window.ui")]
    pub struct MainWindow {
        // Models
        pub stores: OnceCell<Rc<RefCell<Stores>>>,
        pub mime_type_entries: RefCell<Option<gio::ListStore>>,
        pub application_entries: RefCell<Option<gio::ListStore>>,
        pub undo_action: OnceCell<gtk::gio::SimpleAction>,

        // UI bindings
        #[template_child]
        pub commit_button: TemplateChild<Button>,

        #[template_child]
        pub stack: TemplateChild<ViewStack>,

        #[template_child]
        pub mime_types_page: TemplateChild<ViewStackPage>,

        // mime types page UI bindings
        #[template_child]
        pub mime_types_scrolled_window: TemplateChild<ScrolledWindow>,

        #[template_child]
        pub mime_types_list_box: TemplateChild<ListBox>,

        #[template_child]
        pub mime_type_to_application_assignment_scrolled_window: TemplateChild<ScrolledWindow>,

        #[template_child]
        pub mime_type_to_application_assignment_list_box: TemplateChild<ListBox>,

        // applications page UI bindings
        #[template_child]
        pub applications_page: TemplateChild<ViewStackPage>,

        #[template_child]
        pub applications_scrolled_window: TemplateChild<ScrolledWindow>,

        #[template_child]
        pub application_list_box: TemplateChild<ListBox>,

        #[template_child]
        pub application_to_mime_type_assignment_scrolled_window: TemplateChild<ScrolledWindow>,

        #[template_child]
        pub application_to_mime_type_assignment_list_box: TemplateChild<ListBox>,

        //
        pub application_check_button_group: RefCell<Option<CheckButton>>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for MainWindow {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "MainWindow";
        type Type = super::MainWindow;
        type ParentType = gtk::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // Trait shared by all GObjects
    impl ObjectImpl for MainWindow {
        fn constructed(&self) {
            // Call "constructed" on parent
            self.parent_constructed();

            // Setup
            let obj = self.obj();
            obj.setup_models();
            obj.setup_actions();
            obj.setup_ui();
        }
    }

    // Trait shared by all widgets
    impl WidgetImpl for MainWindow {}

    // Trait shared by all windows
    impl WindowImpl for MainWindow {}

    // Trait shared by all application windows
    impl ApplicationWindowImpl for MainWindow {}
}

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

    fn stores(&self) -> Rc<RefCell<Stores>> {
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
        log::debug!("MainWindow::setup_models");

        // Create models
        match Stores::new() {
            Ok(stores) => {
                self.imp()
                    .stores
                    .set(Rc::new(RefCell::new(stores)))
                    .expect("MainWindow::setup_models() should only be set once");
            }
            Err(e) => self.show_error("Uh oh", "Unable to load necessary data", &e),
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
        let borrowed_stores = stores.borrow();
        let mime_associations_store = borrowed_stores.mime_associations_store();

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
        let borrowed_stores = stores.borrow();
        let apps = borrowed_stores.desktop_entry_store();

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

    fn setup_ui(&self) {
        self.imp()
            .commit_button
            .connect_clicked(clone!(@weak self as window => move |_|{
                window.commit_changes();
            }));

        self.setup_mime_types_pane();
        self.setup_applications_pane();
        self.store_was_mutated();
    }

    fn setup_mime_types_pane(&self) {
        log::debug!("MainWindow::setup_mime_types_pane",);

        let mime_types_list_box = &self.imp().mime_types_list_box;
        self.bind_mime_types_pane_model();

        // bind to selection events
        mime_types_list_box.connect_row_activated(clone!(@weak self as window => move |_, row|{
            let index = row.index();
            let model = window.mime_type_entries().item(index as u32)
                .expect("Expected a valid row index")
                .downcast::<MimeTypeEntry>()
                .expect("MainWindow::mime_type_entries() model should contain instances of MimeTypeEntry only");
            window.show_mime_type_to_application_assignment(&model);
        }));

        // Select first entry
        let row = mime_types_list_box.row_at_index(0);
        mime_types_list_box.select_row(row.as_ref());
        self.show_mime_type_to_application_assignment(
            &self
                .mime_type_entries()
                .item(0)
                .expect("Expect non-empty mime type entries model")
                .downcast::<MimeTypeEntry>()
                .expect(
                    "MainWindow::mime_type_entries() model should contain instances of MimeTypeEntry only",
                ));
    }

    /// Binds the `MainWindow::mime_type_entries` list model to the `MainWindow::mime_types_list_view`,
    /// this can be called any time to "reload" the list view contents.
    fn bind_mime_types_pane_model(&self) {
        self.imp().mime_types_list_box.bind_model(
            Some(&self.mime_type_entries()),
            clone!(@weak self as window => @default-panic, move | obj | {
                let model = obj.downcast_ref().unwrap();
                let row = Self::create_mime_type_list_box_row(model);
                row.upcast()
            }),
        );
    }

    fn create_mime_type_list_box_row(model: &MimeTypeEntry) -> ListBoxRow {
        let label = Label::builder()
            .ellipsize(pango::EllipsizeMode::End)
            .xalign(0.0)
            .build();

        let mime_type = &model.mime_type();
        label.set_text(&mime_type.to_string());

        ListBoxRow::builder().child(&label).build()
    }

    fn show_mime_type_to_application_assignment(&self, mime_type_entry: &MimeTypeEntry) {
        log::warn!(
            "MainWindow::show_mime_type_to_application_assignment mime_type_entry: {}",
            mime_type_entry.mime_type(),
        );

        let model = NoSelection::new(Some(mime_type_entry.supported_application_entries()));
        let list_box = &self.imp().mime_type_to_application_assignment_list_box;

        /*
           This approach isn't working. Can't mutate the group object, and I don't now how to pass it
           safely into the closure. One option is to maintain a check_box_group object in the window,
           which can be None. In the create_application_assignment_row method I add the checkbox to that group,
           or create the new group the first time.

           Need to clear that object before bdinding the model.

        */

        // clear the application check button group before building the list
        self.imp()
            .application_check_button_group
            .borrow_mut()
            .take();

        list_box.bind_model(Some(&model),
            clone!(@weak self as window => @default-panic, move |obj| {
                let model = obj.downcast_ref().expect("The object should be of type `ApplicationEntry`.");
                window.create_application_assignment_row(model).upcast()
            }));
    }

    fn create_application_assignment_row(&self, application_entry: &ApplicationEntry) -> ActionRow {
        let check_button = CheckButton::builder()
            .valign(Align::Center)
            .can_focus(false)
            .build();

        let row = ActionRow::builder()
            .activatable_widget(&check_button)
            .build();
        row.add_prefix(&check_button);

        let desktop_entry = application_entry.desktop_entry();
        let title = desktop_entry.name().unwrap_or("<Unnamed Application>");
        row.set_title(title);

        // RadioButtons work by putting check buttons in a group; we check if the group exists
        // and add this check button if it does; otherwise, we need to make a new group from
        // our first check button. It's ugly holding this state in the window, but here we are.
        if let Some(group) = self.imp().application_check_button_group.borrow().as_ref() {
            check_button.set_group(Some(group));
            return row;
        }

        self.imp()
            .application_check_button_group
            .borrow_mut()
            .replace(check_button);

        row
    }

    fn setup_applications_pane(&self) {
        log::debug!("MainWindow::setup_applications_pane",);
        let application_list_box = &self.imp().application_list_box;

        self.bind_applications_pane_model();

        // Listen for selection
        application_list_box.connect_row_activated(clone!(@weak self as window => move |_, row|{
            let index = row.index();
            let model = window.application_entries().item(index as u32)
                .expect("Expected valid item index")
                .downcast::<ApplicationEntry>()
                .expect("MainWindow::application_entries should only contain ApplicationEntry");
            window.show_application_to_mime_type_assignment(&model);
        }));

        // Select first entry
        let row = application_list_box.row_at_index(0);
        application_list_box.select_row(row.as_ref());
        self.show_application_to_mime_type_assignment(
            &self
                .application_entries()
                .item(0)
                .expect("Expect non-empty application entries model")
                .downcast::<ApplicationEntry>()
                .expect("MainWindow::application_entries should only contain ApplicationEntry"),
        );
    }

    /// Binds the `MainWindow::application_entries` list model to the `MainWindow::application_list_box`,
    /// this can be called any time to "reload" the list view contents.
    fn bind_applications_pane_model(&self) {
        self.imp().application_list_box.bind_model(
            Some(&self.application_entries()),
            clone!(@weak self as window => @default-panic, move |obj| {
                let model = obj
                    .downcast_ref()
                    .unwrap();
                let row = Self::create_application_list_box_row(model);
                row.upcast()
            }),
        );
    }

    fn show_application_to_mime_type_assignment(&self, application_entry: &ApplicationEntry) {
        log::debug!(
            "MainWindow::show_application_mime_type_assignment application_entry: {}",
            application_entry.id(),
        );
        let model = NoSelection::new(Some(application_entry.mime_type_assignments()));
        self.imp().application_to_mime_type_assignment_list_box.bind_model(Some(&model),
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

    fn create_application_list_box_row(model: &ApplicationEntry) -> ListBoxRow {
        let label = Label::builder()
            .ellipsize(pango::EllipsizeMode::End)
            .xalign(0.0)
            .build();

        let desktop_entry = &model.desktop_entry();
        label.set_text(desktop_entry.name().unwrap_or("<Unnamed Application>"));

        ListBoxRow::builder().child(&label).build()
    }

    fn setup_actions(&self) {
        log::debug!("MainWindow::setup_actions");

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

        let action_reset_user_default_application_assignments =
            gtk::gio::SimpleAction::new("reset-user-default-applications", None);
        action_reset_user_default_application_assignments.connect_activate(
            clone!(@weak self as window => move |_, _| {
                window.query_reset_user_default_application_assignments();
            }),
        );
        self.add_action(&action_reset_user_default_application_assignments);

        let action_clear_orphaned_application_assignments =
            gtk::gio::SimpleAction::new("prune-orphaned-application-assignments", None);
        action_clear_orphaned_application_assignments.connect_activate(
            clone!(@weak self as window => move |_, _| {
                window.query_prune_orphaned_application_assignments();
            }),
        );
        self.add_action(&action_clear_orphaned_application_assignments);

        let about_action = gtk::gio::SimpleAction::new("show-about", None);
        about_action
            .connect_activate(clone!(@weak self as window => move |_, _| { window.show_about(); }));
        self.add_action(&about_action);

        let discard_uncommited_changes_action =
            gtk::gio::SimpleAction::new("discard-uncommitted-changes", None);
        discard_uncommited_changes_action.connect_activate(
            clone!(@weak self as window => move |_, _| {
                window.discard_uncommitted_changes();
            }),
        );
        self.add_action(&discard_uncommited_changes_action);

        let undo_action = gtk::gio::SimpleAction::new("undo", None);
        undo_action.connect_activate(clone!(@weak self as window => move |_, _| {
            window.undo();
        }));
        self.add_action(&undo_action);
        self.imp()
            .undo_action
            .set(undo_action)
            .expect("MainWindow::setup_actions must only be executed once");
    }

    ///////////////////////////////////////////////////////////////////////////////////////////////////////////////////
    /// User interaction callbacks

    fn assign_application_to_mimetype(
        &self,
        desktop_entry_id: &DesktopEntryId,
        mime_type: &MimeType,
    ) {
        log::debug!(
            "MainWindow::assign_application_to_mimetype application: {} mime_type: {}",
            desktop_entry_id,
            mime_type,
        );

        if let Err(e) = self
            .stores()
            .borrow_mut()
            .assign_application_to_mimetype(desktop_entry_id, mime_type)
        {
            self.show_error("Error", "Unable to assign application to mimetype", &e);
            return;
        }

        // Assignment was successful, mark changes were made
        self.store_was_mutated();
    }

    /// Show user a dialog asking if they want to reset application assignments.
    fn query_reset_user_default_application_assignments(&self) {
        log::debug!("MainWindow::reset_user_default_application_assignments",);

        let cancel_response = "cancel";
        let reset_response = "reset";

        // Create new dialog
        let dialog = adw::MessageDialog::builder()
            .heading("Reset your application handler assignments?")
            .body(
                "Would you like to reset your application handler assignments to system defaults? This will clear out any application assignments you have made.",
            )
            .transient_for(self)
            .modal(true)
            .destroy_with_parent(true)
            .close_response(cancel_response)
            .default_response(reset_response)
            .build();
        dialog.add_responses(&[
            (cancel_response, "Cancel"),
            (reset_response, "Reset to System Defaults"),
        ]);

        dialog.set_response_appearance(reset_response, ResponseAppearance::Destructive);

        dialog.connect_response(
            None,
            clone!(@weak self as window => move |dialog, response|{
                dialog.destroy();
                if response != reset_response {
                    return;
                }

                window.reset_user_default_application_assignments();
            }),
        );

        dialog.present();
    }

    /// Show user a dialog asking if they want to clear orphaned application assignments.
    fn query_prune_orphaned_application_assignments(&self) {
        log::debug!("MainWindow::query_prune_orphaned_application_assignments",);

        let cancel_response = "cancel";
        let clear_response = "clear";

        // Create new dialog
        let dialog = adw::MessageDialog::builder()
            .heading("Clear orphaned application assignments?")
            .body("Would you like to remove any left-over application assignments from uninstalled applications?")
            .transient_for(self)
            .modal(true)
            .destroy_with_parent(true)
            .close_response(cancel_response)
            .default_response(clear_response)
            .build();
        dialog.add_responses(&[(cancel_response, "Cancel"), (clear_response, "Clear")]);

        dialog.set_response_appearance(clear_response, ResponseAppearance::Suggested);

        dialog.connect_response(
            None,
            clone!(@weak self as window => move |dialog, response|{
                dialog.destroy();
                if response != clear_response {
                    return;
                }

                window.prune_orphaned_application_assignments();
            }),
        );

        dialog.present();
    }

    /// Show one of the main window pages
    pub fn show_page(&self, page: MainWindowPage) {
        // Note: we're treating the page selection model as single selection.
        // TODO: Wrap it in a SingleSelection? Is this possible?
        let page_selection_model = self.imp().stack.pages();
        match page {
            MainWindowPage::MimeTypes => {
                log::debug!("MainWindow::show_page - MimeTypes",);
                page_selection_model.select_item(0, true);
            }
            MainWindowPage::Applications => {
                log::debug!("MainWindow::show_page - Applications",);
                page_selection_model.select_item(1, true);
            }
        }
        self.reload_active_page();
    }

    pub fn show_about(&self) {
        let about = adw::AboutWindow::builder()
            .transient_for(self)
            .application_name("MimeAssoc")
            .application_icon(crate::common::APP_ICON)
            .developer_name("Shamyl Zakariya")
            .version(crate::common::APP_VERSION)
            .issue_url("https://github.com/ShamylZakariya/mimeassoc/issues")
            .copyright("© 2023 Shamyl Zakariya")
            .license_type(gtk::License::MitX11)
            .website("https://github.com/ShamylZakariya/mimeassoc")
            .release_notes(
                r#"<ul>
    <li>Nothing to see here, yet.</li>
</ul>"#,
            )
            .build();

        about.add_credit_section(
            Some("Standing on the shoulders of giants"),
            &[
                "GTK https://www.gtk.org/",
                "GNOME https://www.gnome.org/",
                "Libadwaita https://gitlab.gnome.org/GNOME/libadwaita",
                "Workbench https://github.com/sonnyp/Workbench",
                "And many more...",
            ],
        );

        about.present();
    }

    fn reload_active_page(&self) {
        // Note: we're treating the page selection model as single selection
        let page_selection_model = self.imp().stack.pages();
        if page_selection_model.is_selected(0) {
            self.bind_mime_types_pane_model();
        } else if page_selection_model.is_selected(1) {
            self.bind_applications_pane_model();
        } else {
            unreachable!("Somehow the page selection model has a page other than [0,1] selected.")
        }
    }

    fn show_toast(&self, message: &str) {
        log::debug!("MainWindow::show_toast: {}", message,);
    }

    fn show_error(&self, title: &str, message: &str, error: &anyhow::Error) {
        log::error!(
            "MainWindow::show_error title: {}, message: {} error: {:?}",
            title,
            message,
            error
        );
    }

    ///////////////////////////////////////////////////////////////////////////////////////////////

    fn store_was_mutated(&self) {
        let stores = self.stores();
        let stores = stores.borrow();

        let can_undo = stores.can_undo();
        let can_save = stores.is_dirty();

        log::debug!(
            "MainWindow::store_dirty_state_changed is_dirty: {}",
            can_undo,
        );

        self.imp().commit_button.set_visible(can_save);
        self.imp()
            .undo_action
            .get()
            .expect("Expect MainWindow::setup_actions to have run already")
            .set_enabled(can_undo);
    }

    fn discard_uncommitted_changes(&self) {
        log::debug!("MainWindow::discard_uncommitted_changes",);

        let stores = self.stores();
        if let Err(e) = stores.borrow_mut().discard_uncommitted_changes() {
            self.show_error("Error", "Unable to reload mime associations", &e);
        }

        self.store_was_mutated();
        self.reload_active_page();
    }

    fn undo(&self) {
        log::debug!("MainWindow::undo",);

        let stores = self.stores();
        let mut stores = stores.borrow_mut();
        let result = stores.undo();
        drop(stores);

        self.store_was_mutated();
        if let Err(e) = result {
            self.show_error("Oh bother!", "Unable to perform undo", &e);
        } else {
            self.reload_active_page();
        }
    }

    fn reset_user_default_application_assignments(&self) {
        log::debug!("MainWindow::reset_user_default_application_assignments",);

        if let Err(e) = self
            .stores()
            .borrow_mut()
            .reset_user_default_application_assignments()
        {
            self.show_error(
                "Oh no",
                "Unable to reset assigned applications to sytem defaults.",
                &e,
            );
            return;
        }

        // Persist our changes and reload display
        self.show_toast("Application assignments result to system default successfully");
        self.commit_changes();
        self.reload_active_page();
    }

    fn prune_orphaned_application_assignments(&self) {
        log::debug!("MainWindow::clear_orphaned_application_assignments - unimplemented...",);

        if let Err(e) = self
            .stores()
            .borrow_mut()
            .prune_orphaned_application_assignments()
        {
            self.show_error(
                "Oh no",
                "Unable clear out orphaned application assignments.",
                &e,
            );
            return;
        }

        // Persist our changes and reload display
        self.show_toast("Orphaned application assignments cleared successfully");
        self.commit_changes();
        self.reload_active_page();
    }

    fn commit_changes(&self) {
        log::debug!("MainWindow::save_changes");
        if let Err(e) = self.stores().borrow_mut().save() {
            self.show_error("Oh no", "Unable to save changes", &e);
        } else {
            self.show_toast("Committed changes successfully");
        }
        self.store_was_mutated();
    }
}
