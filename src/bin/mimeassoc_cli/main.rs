mod command_output;
mod commands;

use clap::Parser;

use command_output::*;
use commands::*;
use mimeassoc::desktop_entry::*;
use mimeassoc::mime_type::*;
use mimeassoc::*;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Display system mime type and default applications info
#[derive(Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    #[arg(short, long)]
    json: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn process(&self, mime_db: &mut MimeAssociations, desktop_entry_db: &DesktopEntries) {
        if let Some(command) = &self.command {
            let command_output = command.process(mime_db, desktop_entry_db);
            if let Err(e) = self.command_output_consumer().process(&command_output) {
                panic!("Error processing command output: {}", e);
            }
        }
    }

    fn command_output_consumer(&self) -> Box<dyn CommandOutputConsumer> {
        if self.json {
            Box::<JsonCommandOutputConsumer>::default()
        } else {
            Box::<DefaultCommandOutputConsumer>::default()
        }
    }
}

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
