mod application_entry_list_row;
mod main_window;
mod mime_type_entry_list_row;
mod strings;

// re-export for clarity
pub use application_entry_list_row::ApplicationEntryListRow;
pub use main_window::MainWindow;
pub use main_window::MainWindowCommand;
pub use mime_type_entry_list_row::MimeTypeEntryListRow;

use gtk::{glib::*, *};
use traits::AdjustmentExt;
use traits::WidgetExt;

pub fn scroll_listbox_to_selected_row(list_box: ListBox) {
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
            log::debug!(
                "scroll_listbox_to_selected_row coordinates: {:?}",
                coordinates
            );

            // If the window hasn't laid out, we'll have (0,0) coordinates
            if coordinates == (0.0, 0.0) {
                log::debug!(
                    "scroll_listbox_to_selected_row - early exit as we have no height to scroll in"
                );
                return false;
            }

            if let Some(adjustment) = list_box.adjustment() {
                let (_, natural_size) = row.preferred_size();

                log::debug!(
                    "scroll_listbox_to_selected_row adjustment: {:?}, natural_size: {:?}",
                    adjustment,
                    natural_size
                );

                adjustment.set_value(
                    coordinates.1 - (adjustment.page_size() - natural_size.height() as f64) / 2.0,
                );

                return true;
            }
        }
    }
    false
}
