mod mime_assoc;
use mime_assoc::*;

use crate::mime_assoc::desktop_entry::*;
use crate::mime_assoc::mime_type::*;

fn main() {
    let desktop_entry_dirs = match desktop_entry_dirs() {
        Ok(desktop_entry_dirs) => desktop_entry_dirs,
        Err(e) => panic!("Unable to load desktop_entry_dirs: {:?}", e),
    };

    let mimeapps_lists = match mimeapps_lists_paths() {
        Ok(mimeapps_lists) => mimeapps_lists,
        Err(e) => panic!("Unable to load mimeapps_lists_paths: {:?}", e),
    };

    let mime_assoc = match MimeAssociations::load(&mimeapps_lists) {
        Ok(mime_assoc) => mime_assoc,
        Err(e) => panic!("Unable to load MimeAssociations: {:?}", e),
    };

    let desktop_entries = match DesktopEntries::load(&desktop_entry_dirs) {
        Ok(desktop_entries) => desktop_entries,
        Err(e) => panic!("Unable to load DesktopEntries: {:?}", e),
    };

    println!("DESKTOP ENTRY DIRS: ");
    for dir in desktop_entry_dirs.iter() {
        println!("\t{:?}", dir);
    }
    println!("\n\n");

    println!("MIMEAPPS LISTS FILES: ");
    for file in mimeapps_lists.iter() {
        println!("\t{:?}", file);
    }

    println!("\n\nEnumerating default applications for registered mime types:\n");
    // Now, a sanity check in lieu of proper testing
    for mime_type in mime_assoc.mime_types() {
        if let Some(desktop_entry_id) = mime_assoc.default_application_for(mime_type) {
            if let Some(desktop_entry) = desktop_entries.get_desktop_entry(desktop_entry_id) {
                println!(
                    "\tDefault application for MimeType: {} is {}",
                    &mime_type,
                    desktop_entry.name().unwrap_or("<No Name>")
                );
            } else {
                println!(
                    "\tNo default application found for MimeType: {}",
                    &mime_type
                );
            }
        }
    }
}
