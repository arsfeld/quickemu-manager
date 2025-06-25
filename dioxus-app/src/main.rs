use dioxus::prelude::*;

mod api;
mod components;

use components::app::App;

fn main() {
    // Initialize console error reporting for better debugging
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    // Launch the Dioxus web app
    launch(App);
}