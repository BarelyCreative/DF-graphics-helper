#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod structs;
mod error;

use app::DFGraphicsHelper;

pub const PADDING: f32 = 8.0;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            //.with_icon(icon(256x256))
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Dwarf Fortress Graphics Helper",
        native_options,
        Box::new(|cc| Box::new(DFGraphicsHelper::new(cc))),
    )
}