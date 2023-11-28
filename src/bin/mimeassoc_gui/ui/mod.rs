mod application_entry_list_row;
mod main_window;
mod mime_type_entry_list_row;

// re-export for clarity
pub use application_entry_list_row::ApplicationEntryListRow;
pub use main_window::MainWindow;
pub use main_window::MainWindowCommand;
pub use main_window::MainWindowPage;
pub use mime_type_entry_list_row::MimeTypeEntryListRow;

use gtk::{glib::*, *};
use traits::AdjustmentExt;
use traits::WidgetExt;

/// Utility to scroll a ListBox to whichever row is currently selected.
pub fn scroll_listbox_to_selected_row(list_box: ListBox) {
    // Note: glib lays out lazily; if a list box has just been created or populated,
    // we won't get correct layout metrics for scrolling until a layout pass has completed.
    // So, we schedule the layout for next idle tick, and keep trying until we succeed.
    // That being said, try once immediately just in case everything's ready.

    if try_scroll_listbox_to_selected_row(list_box.clone()) {
        return;
    }

    idle_add_local(
        clone!(@weak list_box => @default-return glib::ControlFlow::Break, move || {
            if try_scroll_listbox_to_selected_row(list_box) {
                ControlFlow::Break
            } else {
                ControlFlow::Continue
            }
        }),
    );
}

fn try_scroll_listbox_to_selected_row(list_box: ListBox) -> bool {
    if let Some(row) = list_box.selected_row() {
        if let Some(coordinates) = row.translate_coordinates(&list_box, 0.0, 0.0) {
            // If the window hasn't laid out, we'll have (0,0) coordinates
            if coordinates == (0.0, 0.0) {
                return false;
            }

            if let Some(adjustment) = list_box.adjustment() {
                let (_, natural_size) = row.preferred_size();

                adjustment.set_value(
                    coordinates.1 - (adjustment.page_size() - natural_size.height() as f64) / 2.0,
                );

                return true;
            }
        }
    }
    false
}
