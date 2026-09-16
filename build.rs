fn main() {
    println!("cargo:rerun-if-changed=assets/zenshot.ico");
    embed_windows_icon();
}

#[cfg(windows)]
fn embed_windows_icon() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/zenshot.ico");
    res.set("ProductName", "ZenShot");
    res.set("FileDescription", "ZenShot screen capture");
    res.set("LegalCopyright", "MIT");
    if let Err(err) = res.compile() {
        println!("cargo:warning=winres failed: {err}");
    }
}

#[cfg(not(windows))]
fn embed_windows_icon() {}
