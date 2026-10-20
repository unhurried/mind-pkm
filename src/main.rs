// No console window behind the application on Windows.
#![cfg_attr(not(test), windows_subsystem = "windows")]

use std::path::PathBuf;

fn main() -> iced::Result {
    // The map to open: the first argument (e.g. a .mm file opened with this
    // application from Explorer), else the last opened map.
    let initial = std::env::args_os().nth(1).map(PathBuf::from);
    mind_pkm::app::run(initial)
}
