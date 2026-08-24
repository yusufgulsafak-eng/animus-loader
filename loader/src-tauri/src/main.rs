#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    animus_patch_loader_lib::run();
}
