use crate::{
    raw_handle::RawWaylandHandle,
    types::{Data, PixelSize},
    AssetParams,
};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::{any::Any, collections::HashMap};

/// The trait that backends must implement. An instance is returned by [`current_window`][crate::current_window] so that an app may interact with the OS's windowing system.
pub trait Window: HasRawWindowHandle + HasRawDisplayHandle + Send + Sync + Any {
    /// Logical size of the window. Probably only useful internally.
    fn logical_size(&self) -> PixelSize;

    /// Physical size of the window. Probably only useful internally.
    fn physical_size(&self) -> PixelSize;

    /// Scale factor of the window. Probably only useful internally.
    fn scale_factor(&self) -> f32;

    /// For internal use only.
    fn redraw(&self) {}

    /// Set the current cursor. Cursor names are backend-specific, but they should support the following:
    /// - "Arrow"
    /// - "None"
    /// - "Hidden"
    /// - "Ibeam"
    /// - "Text"
    /// - "PointingHand"
    /// - "Hand"
    /// - "HandGrabbing"
    /// - "NoEntry"
    /// - "Cross"
    /// - "Size"
    /// - "Move"
    /// - "SizeNWSE"
    /// - "SizeNS"
    /// - "SizeNESW"
    /// - "SizeWE"
    fn set_cursor(&self, _cursor_type: &str) {}

    /// Reset the cursor to the default pointer.
    fn unset_cursor(&self) {}

    /// Put the [`Data`] on the clipboard.
    fn put_on_clipboard(&self, _data: &Data) {}

    /// Get the current [`Data`] that is on the clipboard, if any.
    fn get_from_clipboard(&self) -> Option<Data> {
        None
    }

    /// Start a Drag and Drop with the given [`Data`].
    fn start_drag(&self, _data: Data) {}

    /// When responding to a Drag and Drop action, tell the window of origin whether the mouse is currently over a valid drop target.
    fn set_drop_target_valid(&self, _valid: bool) {}

    // For fonts
    fn fonts(&self) -> cosmic_text::fontdb::Database;

    // For assets
    fn assets(&self) -> HashMap<String, AssetParams>;

    // For svgs
    fn svgs(&self) -> HashMap<String, String>;

    // used to reconfigure size
    fn set_size(&mut self, width: u32, height: u32) {}

    // trigger exit
    fn exit(&mut self);

    // used to reconfigure wayland_handle
    fn set_wayland_handle(&mut self, wayland_handle: RawWaylandHandle) {}

    fn has_handle(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any;
}
