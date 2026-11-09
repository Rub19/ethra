fn main() {
    let config = slint_build::CompilerConfiguration::new().with_style("fluent-dark".into()).with_debug_info(std::env::var_os("CARGO_FEATURE_E2E").is_some());
    slint_build::compile_with_config("ui/app.slint", config).unwrap();
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    res.set("ProductName", "Kairo");
    res.set("FileDescription", "Kairo");
    res.compile().unwrap();
}
