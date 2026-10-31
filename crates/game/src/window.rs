//! Native window styling: a dark, translucent title bar on Windows 11.

#[cfg(windows)]
pub fn style_title_bar() {
    use std::ffi::c_void;

    use windows_sys::Win32::Graphics::Dwm::{
        DWMSBT_TRANSIENTWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE,
        DwmSetWindowAttribute,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;

    // SAFETY: the handle comes from the system, and each value outlives its call.
    // Older Windows versions just refuse the attributes, which leaves the stock bar.
    unsafe {
        let window = GetActiveWindow();
        if window.is_null() {
            return;
        }
        let dark: i32 = 1;
        DwmSetWindowAttribute(
            window,
            DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
            (&raw const dark).cast::<c_void>(),
            size_of::<i32>() as u32,
        );
        let backdrop: i32 = DWMSBT_TRANSIENTWINDOW;
        DwmSetWindowAttribute(
            window,
            DWMWA_SYSTEMBACKDROP_TYPE as u32,
            (&raw const backdrop).cast::<c_void>(),
            size_of::<i32>() as u32,
        );
    }
}

#[cfg(not(windows))]
pub fn style_title_bar() {}
