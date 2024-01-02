use std::cell::RefCell;
use std::rc::Rc;

use adw::subclass::prelude::*;
use adw::{prelude::*, *};
use gtk::glib::*;
use mimeassoc::*;

use crate::model::*;
use crate::resources::Strings;
use crate::ui::MainWindow;

use super::{ApplicationsModeController, MimeTypesModeController};

/// Represents a command which can be sent to the main window. This is primarily
/// meant for easing manual testing, but could be used to handle gui cmdline arguments,
/// for example taking the app directly to a specified mime type.
#[derive(Debug)]
pub enum MainWindowCommand {
    ShowMimeType(MimeType),
    ShowApplication(DesktopEntryId),
}

/// Represents the top-level "page" the app is displaying, Applications or Mime Types.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    ApplicationMode,
    MimeTypeMode,
}

mod imp {
    use super::*;
    use std::cell::OnceCell;

    use gtk::glib;

    #[derive(Default)]
    pub struct AppController {
        pub mode: RefCell<Option<Mode>>,
        pub window: OnceCell<WeakRef<MainWindow>>,
        pub stores: OnceCell<Rc<RefCell<Stores>>>,
        pub mime_types_mode_controller: OnceCell<MimeTypesModeController>,
        pub applications_mode_controller: OnceCell<ApplicationsModeController>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for AppController {
        const NAME: &'static str = "AppController";
        type Type = super::AppController;
    }

    // Trait shared by all GObjects
    impl ObjectImpl for AppController {
        fn constructed(&self) {
            Self::parent_constructed(self);
        }
    }
}

glib::wrapper! {
    pub struct AppController(ObjectSubclass<imp::AppController>);
}

impl AppController {
    pub fn new(window: glib::object::WeakRef<MainWindow>) -> Self {
        let instance: AppController = Object::builder().build();

        // basic setup of instance
        instance.imp().window.set(window.clone()).unwrap();
        instance.setup_stores();

        // create view controllers
        let weak_self = glib::object::WeakRef::new();
        weak_self.set(Some(&instance));

        instance
            .imp()
            .mime_types_mode_controller
            .set(MimeTypesModeController::new(
                window.clone(),
                weak_self.clone(),
            ))
            .unwrap();

        instance
            .imp()
            .applications_mode_controller
            .set(ApplicationsModeController::new(window, weak_self))
            .unwrap();

        instance
    }

    pub fn set_mode(&self, mode: Mode) {
        let is_different = match self.imp().mode.borrow().as_ref() {
            Some(current_mode) => *current_mode != mode,
            None => true,
        };

        if !is_different {
            log::debug!("set_mode({:?}) - mode unchanged, skipping", mode);
            return;
        }

        log::debug!("set_mode({:?})", mode);

        let window = self.window();
        match mode {
            Mode::ApplicationMode => {
                self.mime_types_mode_controller().deactivate();
                window.imp().mode_selector_mime_types.set_active(false);

                self.applications_mode_controller().activate();
                window.imp().mode_selector_applications.set_active(true);
            }
            Mode::MimeTypeMode => {
                self.applications_mode_controller().deactivate();
                window.imp().mode_selector_applications.set_active(false);

                self.mime_types_mode_controller().activate();
                window.imp().mode_selector_mime_types.set_active(true);
            }
        }

        self.imp().mode.replace(Some(mode));
    }

    pub fn mode(&self) -> Mode {
        self.imp().mode.borrow().unwrap_or(Mode::ApplicationMode)
    }

    /// Assigns an application to handle a specified mimetype. E.g., assign Firefox to handle text/html
    pub fn assign_application_to_mimetype(
        &self,
        mime_type: &MimeType,
        desktop_entry_id: Option<&DesktopEntryId>,
    ) {
        log::debug!(
            "AppController::assign_application_to_mimetype application: {:?} mime_type: {}",
            desktop_entry_id,
            mime_type,
        );

        if let Err(e) = self
            .stores()
            .borrow_mut()
            .set_application_to_mimetype_binding(mime_type, desktop_entry_id)
        {
            self.show_error("Error", "Unable to assign application to mimetype", &e);
            return;
        }

        // Assignment was successful, mark changes were made
        self.store_was_mutated();
    }

    pub fn reload_active_mode(&self) {
        match self.mode() {
            Mode::ApplicationMode => self.applications_mode_controller().reload(),
            Mode::MimeTypeMode => self.mime_types_mode_controller().reload(),
        }
    }

    pub fn discard_uncommitted_changes(&self) {
        log::debug!("AppController::discard_uncommitted_changes",);

        let stores = self.stores();
        if let Err(e) = stores.borrow_mut().discard_uncommitted_changes() {
            self.show_error("Error", "Unable to reload mime associations", &e);
        }

        self.store_was_mutated();
        self.reload_active_mode();
    }

    pub fn undo(&self) {
        log::debug!("AppController::undo",);

        let stores = self.stores();
        let mut stores = stores.borrow_mut();
        let result = stores.undo();
        drop(stores);

        self.store_was_mutated();
        if let Err(e) = result {
            self.show_error("Oh bother!", "Unable to perform undo", &e);
        } else {
            self.reload_active_mode();
        }
    }

    fn setup_stores(&self) {
        match Stores::new() {
            Ok(stores) => {
                self.imp()
                    .stores
                    .set(Rc::new(RefCell::new(stores)))
                    .expect("AppController::setup_models() should only be set once");
                self.store_was_mutated();
            }
            Err(e) => self.show_error("Uh oh", "Unable to load necessary data", &e),
        }
    }

    fn reset_user_default_application_assignments(&self) {
        log::debug!("AppController::reset_user_default_application_assignments",);

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
        self.reload_active_mode();
    }

    pub fn commit_changes(&self) {
        if let Err(e) = self.stores().borrow_mut().save() {
            self.show_error("Oh no", "Unable to save changes", &e);
        } else {
            self.show_toast("Committed changes successfully");
        }
        self.store_was_mutated();
    }

    fn prune_orphaned_application_assignments(&self) {
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
        self.reload_active_mode();
    }

    fn window(&self) -> MainWindow {
        self.imp()
            .window
            .get()
            .unwrap()
            .upgrade()
            .expect("Expect window instance to be valid")
    }

    pub fn mime_types_mode_controller(&self) -> &MimeTypesModeController {
        self.imp()
            .mime_types_mode_controller
            .get()
            .expect("Expect MimeTypesModeController to be assigned")
    }

    pub fn applications_mode_controller(&self) -> &ApplicationsModeController {
        self.imp()
            .applications_mode_controller
            .get()
            .expect("Expect ApplicationsModeController to be assigned")
    }

    pub fn stores(&self) -> Rc<RefCell<Stores>> {
        self.imp()
            .stores
            .get()
            .expect("Expect Stores instance to have been created")
            .clone()
    }

    fn store_was_mutated(&self) {
        let stores = self.stores();
        let stores = stores.borrow();

        let can_undo = stores.can_undo();
        let can_save = stores.is_dirty();

        log::debug!(
            "AppController::store_was_mutated can_undo: {} can_save: {}",
            can_undo,
            can_save,
        );

        let window = self.window();
        window.imp().commit_button.set_sensitive(can_save);
        window
            .imp()
            .undo_action
            .get()
            .expect("Expect AppController::setup_actions to have run already")
            .set_enabled(can_undo);
    }
}

//
//  UI Callbacks
//

impl AppController {
    /// Show user a dialog asking if they want to reset application assignments.
    pub fn query_reset_user_default_application_assignments(&self) {
        log::debug!("AppController::reset_user_default_application_assignments",);

        let window = self.window();
        let cancel_response = "cancel";
        let reset_response = "reset";

        // Create new dialog
        let dialog = adw::MessageDialog::builder()
            .heading(Strings::reset_user_default_application_assignments_dialog_title())
            .body(Strings::reset_user_default_application_assignments_dialog_body())
            .transient_for(&window)
            .modal(true)
            .destroy_with_parent(true)
            .close_response(cancel_response)
            .default_response(reset_response)
            .build();
        dialog.add_responses(&[
            (cancel_response, Strings::cancel()),
            (
                reset_response,
                Strings::reset_user_default_application_assignments_dialog_action_proceed(),
            ),
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
    pub fn query_prune_orphaned_application_assignments(&self) {
        log::debug!("AppController::query_prune_orphaned_application_assignments",);

        let window = self.window();
        let cancel_response = "cancel";
        let clear_response = "clear";

        // Create new dialog
        let dialog = adw::MessageDialog::builder()
            .heading(Strings::prune_orphaned_application_assignments_dialog_title())
            .body(Strings::prune_orphaned_application_assignments_dialog_body())
            .transient_for(&window)
            .modal(true)
            .destroy_with_parent(true)
            .close_response(cancel_response)
            .default_response(clear_response)
            .build();
        dialog.add_responses(&[
            (cancel_response, Strings::cancel()),
            (
                clear_response,
                Strings::prune_orphaned_application_assignments_dialog_action_proceed(),
            ),
        ]);

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

    pub fn perform_command(&self, command: MainWindowCommand) {
        match command {
            MainWindowCommand::ShowApplication(desktop_entry_id) => {
                self.set_mode(Mode::ApplicationMode);
                self.applications_mode_controller()
                    .select_application(&desktop_entry_id);
            }
            MainWindowCommand::ShowMimeType(mime_type) => {
                self.set_mode(Mode::MimeTypeMode);
                self.mime_types_mode_controller()
                    .select_mime_type(&mime_type);
            }
        }
    }

    pub fn show_about(&self) {
        let window = self.window();
        let about = adw::AboutWindow::builder()
            .transient_for(&window)
            .application_name(crate::common::APP_NAME)
            .application_icon(crate::common::APP_ICON)
            .developer_name(crate::common::APP_DEVELOPER)
            .version(crate::common::APP_VERSION)
            .issue_url(crate::common::APP_ISSUES_URL)
            .copyright(format!("© {}", crate::common::APP_DEVELOPER).as_str())
            .license_type(gtk::License::MitX11)
            .website(crate::common::APP_WEBSITE_URL)
            .release_notes(Strings::about_window_release_notes())
            .build();

        about.add_credit_section(
            Some(Strings::about_window_credits_title()),
            &Strings::about_window_credits(),
        );

        about.present();
    }

    pub fn show_toast(&self, message: &str) {
        log::debug!("AppController::show_toast: {}", message,);
    }

    pub fn show_error(&self, title: &str, message: &str, error: &anyhow::Error) {
        log::error!(
            "AppController::show_error title: {}, message: {} error: {:?}",
            title,
            message,
            error
        );
    }
}
