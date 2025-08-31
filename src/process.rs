use std::fmt::Display;

use xcap::{image::RgbaImage, XCapResult};

type ProcessID = u32;

#[derive(Debug, Clone)]
pub struct Process {
    id: ProcessID,
    name: String,
    window: xcap::Window,
}

impl Process {
    pub fn capture_image(&self) -> XCapResult<RgbaImage> {
        self.window.capture_image()
    }
}

impl PartialEq for Process {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Display for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

pub fn processes() -> Result<Vec<Process>, String> {
    let windows = xcap::Window::all().map_err(|e| e.to_string())?;
    let processes = windows.into_iter().map(process_from_window).collect();
    Ok(processes)
}

fn process_from_window(window: xcap::Window) -> Process {
    let name = if window.title() != "" {
        format!("{} - {}", window.app_name(), window.title())
    } else {
        window.app_name().to_owned()
    };

    Process {
        id: window.id(),
        name,
        window,
    }
}
