// Embeds a manifest so Windows asks for Administrator rights on launch.
// (C:\Windows\Temp and C:\Windows\Prefetch need admin access to be cleaned.)

#[cfg(windows)]
fn main() {
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
</assembly>"#,
    );
    res.compile().expect("failed to embed manifest");
}

#[cfg(not(windows))]
fn main() {}
