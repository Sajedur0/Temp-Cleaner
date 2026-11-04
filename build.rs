// Embeds a manifest so Windows asks for Administrator rights on launch.
// (C:\Windows\Temp and C:\Windows\Prefetch need admin access to be cleaned.)
// Also embeds the application icon and version information.

#[cfg(windows)]
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");

    let mut res = winres::WindowsResource::new();
    res.set_manifest(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </windowsSettings>
  </application>
</assembly>"#,
    );
    res.set_icon("assets/icon.ico");
    res.set("FileDescription", "Temp Cleaner — temporary file cleaner for Windows");
    res.set("ProductName", "Temp Cleaner");
    res.set("InternalName", "TempCleaner");
    res.set("OriginalFilename", "TempCleaner.exe");
    res.set("FileVersion", env!("CARGO_PKG_VERSION"));
    res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
    res.set("LegalCopyright", "Licensed under the Apache License 2.0");
    res.compile().expect("failed to embed Windows resources");
}

#[cfg(not(windows))]
fn main() {}
