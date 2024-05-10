#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use egui::ViewportBuilder;
use eframe::{NativeOptions, run_native, Result};

use df_texture_helper::logic::app::DFGraphicsHelper;

fn main() -> Result<()> { //eframe::Result
    let native_options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_maximized(true)
            //.with_icon(icon(256x256))
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
    run_native(
        "Dwarf Fortress Graphics Helper",
        native_options,
        Box::new(|cc| Box::new(DFGraphicsHelper::new(cc))),
    )
}