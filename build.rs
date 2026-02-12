fn main() {
    glib_build_tools::compile_resources(
        &["src/gui"],
        "src/gui/resources.gresource.xml",
        "compiled.gresource",
    );
}
