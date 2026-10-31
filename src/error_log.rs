//! The error log: crashes and failures to start or run, appended to
//! `mind-pkm-error.log` next to the executable (the application has no console
//! window to print them to).

use std::backtrace::Backtrace;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// The log's file name.
pub const FILE_NAME: &str = "mind-pkm-error.log";
/// The previous log, kept when the log grows past `MAX_SIZE`.
const OLD_FILE_NAME: &str = "mind-pkm-error.log.old";
/// The size past which the next entry starts a new log.
const MAX_SIZE: u64 = 1 << 20;

/// Appends an entry with `text` to the log; never fails or panics.
pub fn write(text: &str) {
    write_in(&dirs(), text);
}

/// Logs panics (of any thread) with their location and a backtrace.
pub fn install() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("(no message)");
        let location = info.location().map(|l| (l.file(), l.line(), l.column()));
        let mut text = describe_panic(std::thread::current().name(), message, location);
        text.push_str("\nbacktrace:\n");
        text.push_str(&Backtrace::force_capture().to_string());
        write(&text);
        previous(info);
    }));
}

/// Where the log goes, in order of preference: next to the executable, else
/// the temporary folder.
#[cfg(not(test))]
fn dirs() -> Vec<PathBuf> {
    let exe_dir = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf));
    exe_dir.into_iter().chain([std::env::temp_dir()]).collect()
}

/// Tests log nowhere, unless a test has the log of its thread redirected.
#[cfg(test)]
fn dirs() -> Vec<PathBuf> {
    TEST_DIR.with(|d| d.borrow().iter().cloned().collect())
}

#[cfg(test)]
thread_local! {
    static TEST_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// Has the log of the current thread written to `dir` (for the rest of the test).
#[cfg(test)]
pub(crate) fn redirect(dir: &Path) {
    TEST_DIR.with(|d| *d.borrow_mut() = Some(dir.to_path_buf()));
}

fn entry(text: &str) -> String {
    let time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    format!("[{time}] mind-pkm {}\n{text}\n\n", env!("CARGO_PKG_VERSION"))
}

/// Appends an entry to the log in `dir`, first keeping a log past `MAX_SIZE` as the old one.
fn append(dir: &Path, text: &str) -> io::Result<()> {
    let path = dir.join(FILE_NAME);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_SIZE) {
        let old = dir.join(OLD_FILE_NAME);
        // Renaming onto an existing file fails on Windows.
        let _ = std::fs::remove_file(&old);
        std::fs::rename(&path, old)?;
    }
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(entry(text).as_bytes())
}

/// Writes to the first of `dirs` that can be written to; if none can, nothing is logged.
fn write_in(dirs: &[PathBuf], text: &str) {
    for dir in dirs {
        if append(dir, text).is_ok() {
            return;
        }
    }
}

fn describe_panic(thread: Option<&str>, message: &str, location: Option<(&str, u32, u32)>) -> String {
    let mut text = format!("panic in thread '{}': {message}", thread.unwrap_or("<unnamed>"));
    if let Some((file, line, column)) = location {
        text.push_str(&format!("\n  at {file}:{line}:{column}"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(dir: &Path) -> String {
        std::fs::read_to_string(dir.join(FILE_NAME)).unwrap()
    }

    #[test]
    fn the_file_is_created_by_the_first_entry() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!dir.path().join(FILE_NAME).exists());
        append(dir.path(), "first").unwrap();
        assert!(dir.path().join(FILE_NAME).exists());
    }

    #[test]
    fn an_entry_has_the_time_the_version_and_the_text() {
        let e = entry("something failed");
        let (head, text) = e.split_once('\n').unwrap();
        assert!(head.starts_with('['), "{head}");
        assert!(head.ends_with(&format!("] mind-pkm {}", env!("CARGO_PKG_VERSION"))), "{head}");
        // `[yyyy-mm-dd hh:mm:ss]`
        assert_eq!(head.find(']'), Some(20), "{head}");
        assert_eq!(text, "something failed\n\n");
    }

    #[test]
    fn entries_are_appended() {
        let dir = tempfile::tempdir().unwrap();
        append(dir.path(), "first").unwrap();
        append(dir.path(), "second").unwrap();
        let log = read(dir.path());
        assert!(log.find("first").unwrap() < log.find("second").unwrap());
        assert_eq!(log.matches("] mind-pkm ").count(), 2);
    }

    #[test]
    fn a_large_log_is_kept_as_old_and_a_new_one_started() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE_NAME), vec![b'x'; MAX_SIZE as usize + 1]).unwrap();
        std::fs::write(dir.path().join(OLD_FILE_NAME), "older").unwrap();
        append(dir.path(), "new").unwrap();
        assert_eq!(std::fs::metadata(dir.path().join(OLD_FILE_NAME)).unwrap().len(), MAX_SIZE + 1);
        let log = read(dir.path());
        assert!(log.contains("new") && log.len() < 200, "{log}");
    }

    #[test]
    fn a_log_at_the_limit_is_appended_to() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(FILE_NAME), vec![b'x'; MAX_SIZE as usize]).unwrap();
        append(dir.path(), "new").unwrap();
        assert!(!dir.path().join(OLD_FILE_NAME).exists());
        assert!(read(dir.path()).ends_with("new\n\n"));
    }

    #[test]
    fn a_folder_that_cannot_be_written_to_falls_back_to_the_next() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing").join("folder");
        let fallback = dir.path().join("fallback");
        std::fs::create_dir(&fallback).unwrap();
        write_in(&[missing.clone(), fallback.clone()], "failed");
        assert!(!missing.exists());
        assert!(read(&fallback).contains("failed"));
    }

    #[test]
    fn nowhere_to_write_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        write_in(&[dir.path().join("missing").join("folder")], "failed");
        write_in(&[], "failed");
    }

    #[test]
    fn a_panic_is_described_with_thread_message_and_location() {
        let text = describe_panic(Some("worker"), "index out of bounds", Some(("src/a.rs", 3, 9)));
        assert!(text.starts_with("panic in thread 'worker': index out of bounds\n  at src/a.rs:3:9"), "{text}");
        let text = describe_panic(None, "boom", None);
        assert!(text.starts_with("panic in thread '<unnamed>': boom"), "{text}");
        assert!(!text.contains("  at "), "{text}");
    }
}
