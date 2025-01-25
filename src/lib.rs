use slint::{ComponentHandle};
use wasm_bindgen::prelude::*;

slint::include_modules!();

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn main() {
    let app = MainApp::new();
    app.run();
}