fn main() -> Result<(), Box<dyn std::error::Error>> {
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/main.gresource.xml",
        "main.gresource",
    );
    Ok(())
}
