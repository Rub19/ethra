fn main() {
    let conf = std::fs::read_to_string("../src-tauri/tauri.conf.json").expect("tauri.conf.json");
    let version = conf.split("\"version\": \"").nth(1).and_then(|r| r.split('"').next()).expect("version");
    println!("cargo:rustc-env=KAIRO_VERSION={version}");
    println!("cargo:rerun-if-changed=../src-tauri/tauri.conf.json");
    println!("cargo:rerun-if-changed=payload/ethra.exe.gz");

    // asInvoker: Windows would otherwise ask for admin rights because the file name contains "setup"
    let manifest = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
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
</assembly>"#;
    let windows = tauri_build::WindowsAttributes::new().window_icon_path("../src-tauri/icons/icon.ico").app_manifest(manifest);
    tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows)).expect("tauri build");
}
