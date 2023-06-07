use std::path::PathBuf;

use mimeassoc::{desktop_entry::DesktopEntryId, mime_type::*};

pub enum CommandOutput {
    MimeTypes(Vec<MimeTypesCommandOutput>),
    MimeType(Vec<MimeTypeCommandOutput>),
    Applications(Vec<ApplicationCommandOutput>),
    Application(ApplicationCommandOutput),
    Set(SetDefaultHandlerCommandOutput),
    Reset(ResetDefaultHandlerCommandOutput),
    Configuration(ConfigurationCommandOutput),
}

pub struct MimeTypesCommandOutput {
    pub mime_type: MimeType,
}

pub struct MimeTypeCommandHandlerInfo {
    pub desktop_entry: DesktopEntryId,
    pub is_default_handler: bool,
}
pub struct MimeTypeCommandOutput {
    pub mime_type: MimeType,
    pub handler_info: Vec<MimeTypeCommandHandlerInfo>,
}

pub struct MimeInfo {
    pub mime_type: MimeType,
    pub is_default_handler: bool,
}

pub struct ApplicationCommandOutput {
    pub desktop_entry: DesktopEntryId,
    pub mime_info: Vec<MimeInfo>,
}

pub struct SetDefaultHandlerCommandOutput {
    pub desktop_entry: DesktopEntryId,
    pub mime_types: Vec<MimeType>,
}

pub struct ResetDefaultHandlerCommandOutput {
    pub mime_types: Vec<MimeType>,
}

pub struct ConfigurationCommandOutput {
    pub mime_association_scope_paths: Vec<PathBuf>,
    pub desktop_entry_scope_paths: Vec<PathBuf>,
}

pub trait CommandOutputConsumer {
    fn process(&self, command_output: &CommandOutput) -> anyhow::Result<()>;
}

pub struct DefaultCommandOutputConsumer {}

impl CommandOutputConsumer for DefaultCommandOutputConsumer {
    fn process(&self, command_output: &CommandOutput) -> anyhow::Result<()> {
        match command_output {
            CommandOutput::MimeTypes(output) => Self::display_mime_types(output),
            CommandOutput::MimeType(output) => Self::display_mime_type(output),
            CommandOutput::Applications(output) => Self::display_applications(output),
            CommandOutput::Application(output) => Self::display_application(output),
            CommandOutput::Set(output) => Self::display_set(output),
            CommandOutput::Reset(output) => Self::display_reset(output),
            CommandOutput::Configuration(output) => Self::display_configuration(output),
        }
        Ok(())
    }
}

impl DefaultCommandOutputConsumer {
    fn display_mime_types(output: &[MimeTypesCommandOutput]) {
        for mime_type in output.iter() {
            println!("{}", mime_type.mime_type);
        }
    }

    fn display_mime_type(output: &[MimeTypeCommandOutput]) {
        for entry in output.iter() {
            println!("{}:", entry.mime_type);
            for handler in entry.handler_info.iter() {
                if handler.is_default_handler {
                    println!("\t*{}", handler.desktop_entry);
                } else {
                    println!("\t {}", handler.desktop_entry);
                }
            }
            println!();
        }
    }

    fn display_applications(output: &[ApplicationCommandOutput]) {
        for info in output.iter() {
            Self::display_application(info);
            println!("\n");
        }
    }

    fn display_application(output: &ApplicationCommandOutput) {
        println!("{}", output.desktop_entry);
        for mime_info in output.mime_info.iter() {
            if mime_info.is_default_handler {
                println!("\t*{}", mime_info.mime_type);
            } else {
                println!("\t {}", mime_info.mime_type);
            }
        }
    }

    fn display_set(output: &SetDefaultHandlerCommandOutput) {
        if output.mime_types.is_empty() {
            println!("No mime types were assigned to {}", output.desktop_entry);
        } else {
            println!("Assigned {} to:", output.desktop_entry);
            for mime_type in output.mime_types.iter() {
                println!("\t{}", mime_type)
            }
        }
    }

    fn display_reset(output: &ResetDefaultHandlerCommandOutput) {
        if output.mime_types.is_empty() {
            println!("No mimetypes were reset.");
        } else {
            println!(
                "Reset {}",
                output
                    .mime_types
                    .iter()
                    .map(|m| m.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    fn display_configuration(output: &ConfigurationCommandOutput) {
        println!("mimeapps.lists:");
        for path in output.mime_association_scope_paths.iter() {
            if let Some(path) = path.to_str() {
                println!("\t{}", path);
            }
        }
        println!("\nDesktop entry dirs:");
        for path in output.desktop_entry_scope_paths.iter() {
            if let Some(path) = path.to_str() {
                println!("\t{}", path);
            }
        }
    }
}
