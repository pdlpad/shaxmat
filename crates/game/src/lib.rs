//! The SHAXMAT game. Everything but `render` and `main` is free of graphics and
//! input, and so testable without a window.

pub mod app;
pub mod bot;
pub mod layout;
pub mod render;
pub mod session;

#[cfg(not(target_arch = "wasm32"))]
pub mod demo;
