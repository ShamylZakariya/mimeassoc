mod cli;
mod command_output;

use clap::Parser;

use cli::*;
use mimeassoc::desktop_entry::*;
use mimeassoc::mime_type::*;
use mimeassoc::*;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

fn main() {
    let desktop_entry_dirs = match desktop_entry_dirs() {
        Ok(desktop_entry_dirs) => desktop_entry_dirs,
        Err(e) => panic!("Unable to load desktop_entry_dirs: {:?}", e),
    };

    let mimeapps_lists = match mimeapps_lists_paths() {
        Ok(mimeapps_lists) => mimeapps_lists,
        Err(e) => panic!("Unable to load mimeapps_lists_paths: {:?}", e),
    };

    let mut mime_db = match MimeAssociations::load(&mimeapps_lists) {
        Ok(mimeassoc) => mimeassoc,
        Err(e) => panic!("Unable to load MimeAssociations: {:?}", e),
    };

    let desktop_entry_db = match DesktopEntries::load(&desktop_entry_dirs) {
        Ok(desktop_entries) => desktop_entries,
        Err(e) => panic!("Unable to load DesktopEntries: {:?}", e),
    };

    let cli = Cli::parse();
    cli.process(&mut mime_db, &desktop_entry_db);
}
