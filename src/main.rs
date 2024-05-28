#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use egui::ViewportBuilder;
use eframe::{NativeOptions, run_native, Result};

use df_texture_helper::logic::app::DFGraphicsHelper;

fn load_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("../icons/DFGH_icon.png");
    let image = image::load_from_memory(icon_bytes).unwrap();
    let image = image.to_rgba8();
    let (width, height) = image.dimensions();
    
    egui::IconData {
        width,
        height,
        rgba: image.as_raw().to_vec(),
    }
}

fn main() -> Result<()> { //eframe::Result
    let native_options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_maximized(true)
            .with_icon(load_icon())
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
    run_native(
        "DF Graphics Helper",
        native_options,
        Box::new(|cc| Box::new(DFGraphicsHelper::new(cc))),
    )
}