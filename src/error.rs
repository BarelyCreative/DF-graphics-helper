use egui::Context;

use crate::{PADDING, app};
use app::{DFGraphicsHelper, MainWindow};

pub type Result<T> = std::result::Result<T, DFGHError>;

//Dwarf Fortress Graphics Helper Error
#[derive(Debug, thiserror::Error)]
pub enum DFGHError {
    #[error("No error")]
    None,

    #[error("An intermittent error occured while reading or saving to a file.\nPlease try again")]
    IoError(#[from] std::io::Error),

    #[error("An error occured while processing an image.")]
    ImageError(#[from] image::ImageError),

    #[error("An error occured while attempting to import line {0}:\n\t\"{1}\"\n\nIn file:\n\t{2}")]
    ImportError(usize, String, std::path::PathBuf),

    #[error("Failed to parse an integer while importing a string.")]
    ImportParseError(#[from] std::num::ParseIntError),

    #[error("An error occured while attempting to import condition:\n\t{0}")]
    ImportConditionError(String),

    #[error("No valid directory found at path:\n\t{0}\n\nFormat is <mod_name (numeric version)>/graphics/")]
    NoGraphicsDirectory(std::path::PathBuf),

    #[error("File name includes unsupported characters(non UTF-8):\n\t{0}")]
    UnsupportedFileName(std::path::PathBuf),

    #[error("Index out of bounds")]
    IndexError,
}
impl DFGHError {
    pub const fn is_ok(&self) -> bool {
        matches!(*self, DFGHError::None)
    }
}

pub fn error_window(state: &mut DFGraphicsHelper, ctx: &Context) {
    egui::Window::new("Error Window")
        .collapsible(false)
        .constrain(true)
        .title_bar(false)
        .default_size([600.0, 200.0])
        .show(ctx, |ui| {

        egui::TopBottomPanel::top("exception panel")
            .show_separator_line(false)
            .show_inside(ui, |ui| {
            ui.label("Error:");
            ui.separator();
            ui.label(state.exception.to_string());
        });

        egui::TopBottomPanel::bottom("Ok")
            .min_height(20.0)
            .show_inside(ui, |ui| {
            ui.add_space(PADDING);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                if ui.button("      Ok      ").clicked() {
                    match state.exception {
                        DFGHError::IoError(..) => {
                            state.exception = DFGHError::None;
                        },
                        DFGHError::ImageError(..) => {
                            state.undo();
                            state.exception = DFGHError::None;
                        },
                        DFGHError::ImportError(..) => {
                            state.exception = DFGHError::None;
                        },
                        DFGHError::ImportParseError(..) => {
                            state.exception = DFGHError::None;
                        },
                        DFGHError::ImportConditionError(..) => {
                            state.exception = DFGHError::None;
                        },
                        DFGHError::NoGraphicsDirectory(..) => {
                            state.exception = DFGHError::None;
                        },
                        DFGHError::UnsupportedFileName(..) => {
                            state.exception = DFGHError::None;
                        },
                        DFGHError::IndexError => {
                            state.undo();
                            state.main_window = MainWindow::DefaultMenu;
                            state.indices = [0, 0, 0, 0, 0, 0, 0, 0 as usize].into();
                            state.exception = DFGHError::None;
                        },
                        DFGHError::None => {},
                    }
                }
            });
        });
    });
}