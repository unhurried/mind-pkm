// Embeds the application icon (and version info) into the Windows executable,
// so Explorer and the taskbar show it.
fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            panic!("failed to embed the application icon: {e}");
        }
    }
}
