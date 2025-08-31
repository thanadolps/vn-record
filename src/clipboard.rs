use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

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

pub fn write_file_uri(path: &Path) {
    let uri = format!("file://{}", path.canonicalize().unwrap().to_string_lossy());
    let mut cmd = Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .arg("-t")
        .arg("text/uri-list")
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    cmd.stdin
        .as_mut()
        .unwrap()
        .write_all(uri.as_bytes())
        .unwrap();
    let res = cmd.wait().unwrap();

    if !res.success() {
        eprintln!("Failed to write audio to clipboard");
    }
}
