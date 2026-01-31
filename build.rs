fn main() {
    glib_build_tools::compile_resources(
        &["src/gui/v4"],
        "src/gui/v4/resources.gresource.xml",
        "compiled.gresource",
    );
}
