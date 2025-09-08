use std::{path::Path, process::Command};

use duct::cmd;

#[deprecated]
pub fn write_image(path: &Path) {
    let res = Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .arg("-t")
        .arg("image/png")
        .arg("-i")
        .arg(path)
        .status()
        .unwrap();

    if !res.success() {
        eprintln!("Failed to write image to clipboard");
    }
}

pub fn write_file_uris(paths: &[impl AsRef<Path>]) {
    let uri = paths
        .iter()
        .map(|p| {
            format!(
                "file://{}",
                p.as_ref().canonicalize().unwrap().to_string_lossy()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let res = cmd("xclip", ["-selection", "clipboard", "-t", "text/uri-list"])
        .stdin_bytes(uri.as_bytes())
        .run()
        .unwrap();

    if !res.status.success() {
        eprintln!("Failed to write file uris to clipboard");
    }
}
