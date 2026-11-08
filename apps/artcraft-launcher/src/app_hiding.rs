//! macOS only: putting the launcher away in the menu bar the way macOS hides an app (⌘H).
//!
//! A window that is only ordered out can't be brought back from the Dock: clicking the Dock icon
//! of a running app sends it a "reopen", which AppKit answers by restoring minimized windows only,
//! and catching it needs an app delegate (unsafe code). A hidden app comes back with its windows
//! on a Dock click or ⌘-Tab by itself, so the launcher hides itself instead.

use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;

/// Hides the launcher and its windows. Must run on the main thread, as the UI does; elsewhere it
/// does nothing.
pub fn hide() {
    let Some(mtm) = MainThreadMarker::new() else { return };
    NSApplication::sharedApplication(mtm).hide(None);
}

/// Shows the launcher's windows again after [`hide`].
pub fn unhide() {
    let Some(mtm) = MainThreadMarker::new() else { return };
    NSApplication::sharedApplication(mtm).unhide(None);
}

/// Whether the launcher is hidden: after [`hide`], until [`unhide`], a Dock click or ⌘-Tab.
pub fn is_hidden() -> bool {
    MainThreadMarker::new().is_some_and(|mtm| NSApplication::sharedApplication(mtm).isHidden())
}
