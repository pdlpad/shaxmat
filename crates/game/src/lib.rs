//! The SHAXMAT game. Everything but `assets`, `render` and `main` is free of graphics and
//! input, and so testable without a window.

pub mod app;
pub mod assets;
pub mod bot;
pub mod effects;
pub mod i18n;
pub mod layout;
pub mod palette;
pub mod render;
pub mod session;

#[cfg(not(target_arch = "wasm32"))]
pub mod demo;
