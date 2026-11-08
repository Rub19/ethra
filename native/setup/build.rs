fn main() {
    let conf = std::fs::read_to_string("../Cargo.toml").expect("Cargo.toml");
    let version = conf.split("version = \"").nth(1).and_then(|r| r.split('"').next()).expect("version");
    println!("cargo:rustc-env=KAIRO_VERSION={version}");
    println!("cargo:rerun-if-changed=../Cargo.toml");
    println!("cargo:rerun-if-changed=payload/ethra.exe.gz");
    slint_build::compile("ui/setup.slint").unwrap();
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../assets/icon.ico");
    res.set("ProductName", "Kairo");
    res.set("FileDescription", "Installation de Kairo");
    res.set_manifest(
        r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*" />
    </dependentAssembly>
  </dependency>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>"#,
    );
    res.compile().unwrap();
}
