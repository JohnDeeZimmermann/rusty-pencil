//! What differs between the desktop and the browser: where app data lives, how the user picks
//! and receives files, and which keys the host reserves. Everything above this module shares one
//! code path; only the two backends know about `std::fs` or the DOM.
//!
//! App data (recovery scenes, the workspace manifest, preferences, the library) is addressed by
//! path under [`data_dir`]. On native those are files written atomically; in the browser they are
//! keys in IndexedDB, mirrored in memory so reads stay synchronous.
//!
//! User files are picked as a name plus bytes ([`Picked`]). On native the name is the full path;
//! in the browser it is only the file name, and saving a file means downloading it.

use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::*;

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub(crate) use web::*;

/// A file the user chose: where it came from and its contents.
#[derive(Debug, Clone)]
pub(crate) struct Picked {
    /// A full path on native; only the file name in the browser.
    pub(crate) path: PathBuf,
    pub(crate) bytes: Vec<u8>,
}

/// Shortcut labels for keys whose desktop binding the browser keeps for itself.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TabKeys {
    pub(crate) new: &'static str,
    pub(crate) close: &'static str,
    pub(crate) next: &'static str,
}

/// Whether tab shortcuts use Alt (browser) instead of the command key (desktop). Browsers never
/// deliver Ctrl+T, Ctrl+W, Ctrl+N or Ctrl+Tab to a page.
pub(crate) const TAB_KEYS_USE_ALT: bool = cfg!(target_arch = "wasm32");

pub(crate) const TAB_KEYS: TabKeys = if TAB_KEYS_USE_ALT {
    TabKeys {
        new: "Alt+T",
        close: "Alt+W",
        // Alt+Tab belongs to the desktop; PageUp/PageDown cycle instead.
        next: "Alt+PageDown",
    }
} else {
    TabKeys {
        new: "Ctrl+T",
        close: "Ctrl+W",
        next: "Ctrl+Tab",
    }
};

#[cfg(test)]
mod tests;
