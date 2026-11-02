//! Desktop backend: app data and user files are ordinary files, written atomically.
use super::Picked;
use crate::State;
use rune::{FileDialogOptions, Tx};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

/// Verb for a finished save, as the status line shows it.
pub(crate) const SAVED: &str = "Saved";

/// The per-user data directory. The folder keeps its pre-rename name so recovery, preferences
/// and the library survive the rename to rusty-pencil.
pub(crate) fn data_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "drusty-pen").map(|p| p.data_local_dir().to_owned())
}

pub(crate) fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| e.to_string())
}

pub(crate) fn exists(path: &Path) -> bool {
    path.exists()
}

/// Replaces `path` atomically: a crash mid-write leaves the old file, never half a new one.
pub(crate) fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Removes app data; data that is already gone is not an error.
pub(crate) fn remove(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.to_string()),
        _ => Ok(()),
    }
}

/// Writes a file the user chose to save.
pub(crate) fn deliver(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write(path, bytes)
}

/// Whether two user-file paths name the same file, so a file opens in one tab only.
pub(crate) fn same_file(a: &Path, b: &Path) -> bool {
    a == b
}

pub(crate) fn file_identity(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        match (parent.canonicalize(), path.file_name()) {
            (Ok(parent), Some(name)) => parent.join(name),
            _ => path.to_owned(),
        }
    })
}

/// Desktop recents reopen the original file, so no separate scene copy is needed.
pub(crate) fn cache_recent(
    _: &Path,
    _: &Path,
    _: &pen_core::file::SceneFile,
) -> Result<(), String> {
    Ok(())
}

pub(crate) fn recent_source(_: &Path, path: &Path) -> PathBuf {
    path.to_owned()
}

pub(crate) fn remove_recent_copy(_: &Path, _: &Path) -> Result<(), String> {
    Ok(())
}

pub(crate) fn recent_location(path: &Path) -> String {
    let parent = path.parent().unwrap_or(path);
    // Keep the distinguishing folder visible; the row's tooltip carries the full path.
    parent.file_name().map_or_else(
        || parent.display().to_string(),
        |folder| format!("…/{}", folder.to_string_lossy()),
    )
}

/// Asks for a file to open and reads it; `then` gets `Ok(None)` when the user cancels.
pub(crate) fn open_file(
    tx: &mut Tx,
    options: FileDialogOptions,
    then: impl FnOnce(&mut State, Result<Option<Picked>, String>, &mut Tx) + Send + 'static,
) {
    tx.open_file(options, move |s: &mut State, result, tx| {
        let picked = match result {
            Ok(Some(path)) => read(&path).map(|bytes| Some(Picked { path, bytes })),
            Ok(None) => Ok(None),
            Err(error) => Err(error.to_string()),
        };
        then(s, picked, tx);
    });
}

/// Asks where to save; `then` gets the chosen path, or `Ok(None)` when the user cancels.
pub(crate) fn save_file(
    _: &mut State,
    tx: &mut Tx,
    options: FileDialogOptions,
    then: impl FnOnce(&mut State, Result<Option<PathBuf>, String>, &mut Tx) + Send + 'static,
) {
    tx.save_file(options, move |s: &mut State, result, tx| {
        then(s, result.map_err(|e| e.to_string()), tx);
    });
}
