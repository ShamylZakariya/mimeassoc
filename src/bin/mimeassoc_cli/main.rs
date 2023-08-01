mod command_output;
mod commands;

use clap::Parser;
use log::{Level, LevelFilter, Metadata, Record};

use command_output::*;
use commands::*;
use mimeassoc::*;

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Display system mime type and default applications info
#[derive(Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    /// If set, produce all output in JSON
    #[arg(short, long)]
    json: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn process(
        &self,
        mime_associations_store: &mut MimeAssociationStore,
        desktop_entry_store: &DesktopEntryStore,
        mime_info_store: &MimeInfoStore,
    ) {
        if let Some(command) = &self.command {
            let command_output = command.process(
                mime_associations_store,
                desktop_entry_store,
                mime_info_store,
            );
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

struct StdoutLogger {
    threshold: Level,
}

impl log::Log for StdoutLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.threshold
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!(
                "{} - {} - {}",
                LIB_LOG_DOMAIN,
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

static LOGGER: StdoutLogger = StdoutLogger {
    threshold: Level::Debug,
};

////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

fn main() {
    let log_level_filter = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Off
    };

    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log_level_filter))
        .expect("Expect to set up logger");

    let desktop_entry_dirs = match desktop_entry_dirs() {
        Ok(desktop_entry_dirs) => desktop_entry_dirs,
        Err(e) => panic!("Unable to load desktop_entry_dirs: {:?}", e),
    };

    let mimeapps_lists = match mimeapps_lists_paths() {
        Ok(mimeapps_lists) => mimeapps_lists,
        Err(e) => panic!("Unable to load mimeapps_lists_paths: {:?}", e),
    };

    let mimetype_info_paths = match mimeinfo_paths() {
        Ok(paths) => paths,
        Err(e) => panic!("Unable to load mimeinfo_paths: {:?}", e),
    };

    let mut mime_associations_store = match MimeAssociationStore::load(&mimeapps_lists) {
        Ok(mimeassoc) => mimeassoc,
        Err(e) => panic!("Unable to load MimeAssociationStore: {:?}", e),
    };

    let desktop_entry_store = match DesktopEntryStore::load(&desktop_entry_dirs) {
        Ok(desktop_entries) => desktop_entries,
        Err(e) => panic!("Unable to load DesktopEntryStore: {:?}", e),
    };

    let mime_info_store = match MimeInfoStore::load(&mimetype_info_paths) {
        Ok(s) => s,
        Err(e) => panic!("Unable to load MimeTypeInfoStore: {:?}", e),
    };

    let cli = Cli::parse();
    cli.process(
        &mut mime_associations_store,
        &desktop_entry_store,
        &mime_info_store,
    );
}
