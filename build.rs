fn main() {
    glib_build_tools::compile_resources(
        &["src/bin/mimeassoc_gui/resources"],
        "src/bin/mimeassoc_gui/resources/resources.gresource.xml",
        "mimeassoc.gresource",
    );
}
