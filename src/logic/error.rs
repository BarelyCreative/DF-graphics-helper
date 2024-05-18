use std::path;
use std::ops::RangeInclusive;

use egui::Context;

use super::super::PADDING;
use super::app;
use app::{DFGraphicsHelper, MainWindow};

pub type Result<T> = std::result::Result<T, DFGHError>;

//Dwarf Fortress Graphics Helper Error
#[derive(Debug, thiserror::Error)]
pub enum DFGHError {
    #[error("Unexpected error type. This error is not intended to be displayed.")]
    None,

    #[error("Index out of bounds")]
    IndexError,

    #[error("Failed to read or save a file. Please check if the file is in use and try again.")]
    IoError(#[from] std::io::Error),

    #[error("Failed to process an image.")]
    ImageError(#[from] image::ImageError),

    #[error("Unkown import error.")]
    ImportUnknownError,

    #[error("Expected integer in the marked field.")]
    ImportParseError(#[from] std::num::ParseIntError),

    #[error("Expected {0} fields, found {1}")]
    ImportIndexError(usize, usize),

    #[error("File name includes unsupported characters(non UTF-8):\n\t{0}")]
    UnsupportedFileName(std::path::PathBuf),

    #[error("No valid directory found at:\n\t{0}\n\nFormat is mod_name (numeric version)/graphics/")]
    NoGraphicsDirectory(std::path::PathBuf),

    #[error("Should not be displayed. {4}")]
    ImportBlockError(usize, usize, RangeInclusive<usize>, Vec<String>, String),

    #[error("Failed to import line {0} in file:\n\n{2}\n\n{1}\n\n{3}")]
    ImportError(usize, String, std::path::PathBuf, String),
}

fn highlight_error(raw_buffer: Vec<String>, i_b_line: usize, r_elem: RangeInclusive<usize>) -> String {
    let mut highlighted = String::new();
    
    for (i_line, raw_line) in raw_buffer.iter().enumerate() {
        if i_line.ne(&i_b_line) {
            highlighted.push_str(&raw_line);
            highlighted.push('\n');
        } else {
            let breaks = raw_line
                .chars()
                .enumerate()
                .filter(|(_, c)| *c == '[' || *c == ']' || *c == ':')
                .map(|(i, _)| i)
                .collect::<Vec<usize>>();

            dbg!(raw_buffer.clone());
            dbg!(breaks.clone());
            dbg!(i_b_line.clone());
            dbg!(r_elem.clone());
            let highlight: String = raw_line.chars().enumerate()
                .map(|(i, c)| 
                if i < breaks[*r_elem.start()] {
                    if c == '\t' {
                        '\t'
                    } else {
                        ' '
                    }
                } else if i <= breaks[*r_elem.end() + 1] {
                    '^'
                } else {
                    ' '
                }
                )
                .collect::<String>();

            highlighted.push_str(&raw_line);
            highlighted.push('\n');
            highlighted.push_str(&highlight);
            highlighted.push('\n');

        }
    }
    //remove last newline
    highlighted.pop();

    highlighted
}

pub fn wrap_import_error<T>(e: DFGHError, buffer_start: usize, path: &path::PathBuf) -> Result<T> {
    match e {
        DFGHError::ImportBlockError(i_line, i_b_line, r_elem, raw_buffer, e_string) => {
            dbg!(&i_line+&i_b_line);
            Err(
                DFGHError::ImportError(
                    buffer_start + i_line,
                    highlight_error(raw_buffer, i_b_line, r_elem),
                    path.to_path_buf(),
                    e_string,
                )
            )
        },
        _ => Err(DFGHError::ImportUnknownError),
    }
}

pub fn wrap_block_error<T>(i_b_line: usize, r_elem: RangeInclusive<usize>, raw_buffer: Vec<String>, e: DFGHError) -> Result<T> {
    match e {
        DFGHError::IoError(_) |
        DFGHError::ImageError(_) |
        DFGHError::ImportParseError(_) |
        DFGHError::UnsupportedFileName(_) => {
            dbg!("block wrapping outside error.");
            dbg!(&i_b_line);
            dbg!(&r_elem);
            Err(
                DFGHError::ImportBlockError(i_b_line, i_b_line, r_elem, raw_buffer, e.to_string())
            )
        },
        DFGHError::ImportBlockError(i_l, i_b, r, r_b, e_string) => {
            dbg!("block re-wrapping block");
            dbg!(&i_l);
            dbg!(&i_l + &i_b_line);
            Err(
                DFGHError::ImportBlockError(i_l + i_b_line, i_b, r, r_b, e_string)
            )
        },
        _ => Err(DFGHError::ImportUnknownError),
    }
}

pub fn error_window(state: &mut DFGraphicsHelper, ctx: &Context) {
    egui::Window::new("Error Window")
        .collapsible(false)
        .constrain(true)
        .title_bar(false)
        .default_size([600.0, 200.0])
        .show(ctx, |ui| {

        egui::ScrollArea::both()
            .show(ui, |ui| {
                ui.label("Error:");
                ui.separator();
                ui.label(egui::RichText::new(state.exception.to_string()).monospace());
            }
        );

        //button to accept error and potentially attempt to correct.
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
                        DFGHError::ImportBlockError(..) => {
                            state.exception = DFGHError::None;
                        },
                        DFGHError::ImportParseError(..) => {
                            state.exception = DFGHError::None;
                        },
                        DFGHError::NoGraphicsDirectory(..) => {
                            state.exception = DFGHError::None;
                        },
                        DFGHError::UnsupportedFileName(..) => {
                            state.exception = DFGHError::None;
                        },
                        DFGHError::ImportIndexError(..) => {
                            state.exception = DFGHError::None;
                        },
                        DFGHError::ImportUnknownError => {
                            state.exception = DFGHError::None;
                        },
                        DFGHError::IndexError => {
                            state.undo();
                            state.main_window = MainWindow::DefaultMenu;
                            state.indices = [0, 0, 0, 0, 0, 0, 0, 0 as usize].into();
                            state.exception = DFGHError::None;
                        },
                        DFGHError::None => {}
                    }
                }
            });
        });
    });
}