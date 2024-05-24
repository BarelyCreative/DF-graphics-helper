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

    #[error("Unable to locate an image at the current path:\n\t{0}")]
    ImageLoadError(path::PathBuf),

    #[error("Unknown import error.")]
    ImportUnknownError,

    #[error("File contains incompatible tags.")]
    ImportMismatchError,

    #[error("Expected integer in the marked field.")]
    ImportParseError(#[from] std::num::ParseIntError),

    #[error("Expected {0} fields, found {1}")]
    ImportIndexError(usize, usize),

    #[error("File name includes unsupported characters(non UTF-8):\n\t{0}")]
    UnsupportedFileName(std::path::PathBuf),

    #[error("No valid directory found at:\n\t{0}\n\nFormat is mod_name (numeric version)/graphics/")]
    NoGraphicsDirectory(std::path::PathBuf),

    #[error("Should not be displayed. Error String: {3}")]
    ImportBufferError(usize, usize, RangeInclusive<usize>, String),

    #[error("Failed to import line {0} in file:\n\n{1}\n\n{2}\n\n{3}")]
    ImportError(usize, std::path::PathBuf, String, String),
}

//todo make fn truncate before and after relevant line
fn highlight_error(raw_buffer: Vec<String>, i_line: usize, r_error: RangeInclusive<usize>) -> String {
    let mut highlighted = String::new();
    let display_range = 8;

    if i_line > display_range {
        highlighted.push_str("...")
    }
    
    for (i, raw_line) in raw_buffer.iter().enumerate() {
        if i.abs_diff(i_line) <= display_range && i.ne(&i_line) {
            highlighted.push('\n');
            highlighted.push_str(&raw_line);
        } else if i.eq(&i_line) {
            let breaks = raw_line
                .chars()
                .enumerate()
                .filter(|(_, c)| *c == '[' || *c == ']' || *c == ':')
                .map(|(i, _)| i)
                .collect::<Vec<usize>>();

            let mut highlight = String::new();
            if breaks.len() >= 2 {
                let (start, end);
                let breaks_end = breaks.len().saturating_sub(1);
                if r_error.start() > &breaks_end {
                    (start, end) = (0, breaks_end);
                } else if r_error.end() > &breaks_end {
                    (start, end) = (*r_error.start(), breaks_end);
                } else {
                    (start, end) = (*r_error.start(), *r_error.end());
                }
                for (i_highlight, char) in raw_line.chars().enumerate() {
                    if (breaks[start]..=breaks[end]).contains(&i_highlight) {
                        highlight.push_str("^");
                    } else if char == '\t' {
                        highlight.push_str("\t");
                    } else {
                        highlight.push_str(" ");
                    }
                }
            }
            highlighted.push('\n');
            highlighted.push_str(&raw_line);
            highlighted.push('\n');
            highlighted.push_str(&highlight);
        }
    }

    if (raw_buffer.len().abs_diff(i_line)) > display_range {
        highlighted.push_str("\n...")
    }

    highlighted
}

pub fn wrap_import_file_error<T>(raw_buffer: Vec<String>, e: DFGHError, i_line: usize, path: &path::PathBuf) -> Result<T> {
    match e {
        DFGHError::ImportBufferError(i_rel_line, buffer_len, r_error, error_string) => {
            let line_number = (i_line + i_rel_line).saturating_sub(buffer_len);
            dbg!(format!("w_I {}", line_number));
            dbg!(i_rel_line);
            dbg!(buffer_len);
            dbg!(i_line);
            Err(
                DFGHError::ImportError(
                    line_number + 1,
                    path.to_path_buf(),
                    highlight_error(raw_buffer, line_number, r_error),
                    error_string,
                )
            )
        },
        _ => Err(DFGHError::ImportUnknownError),
    }
}

pub fn wrap_import_buffer_error<T>(i_rel_line: usize,  buffer_len: usize, r_error: RangeInclusive<usize>, e: DFGHError) -> Result<T> {
    match e {
        DFGHError::IoError(_) |
        DFGHError::ImageError(_) |
        DFGHError::ImportParseError(_) |
        DFGHError::ImportMismatchError |
        DFGHError::UnsupportedFileName(_) => {
            dbg!(format!("w_e {}", i_rel_line));
            dbg!(i_rel_line);
            dbg!(buffer_len);
            Err(
                DFGHError::ImportBufferError(i_rel_line, buffer_len, r_error, e.to_string())
            )
        },
        DFGHError::ImportBufferError(i_rel, b_len, r_e, e_string) => {
            let line_number = (i_rel_line + i_rel + 1).saturating_sub(b_len);
            dbg!(format!("w_B {}", line_number));
            dbg!(i_rel);
            dbg!(b_len);
            dbg!(i_rel_line);
            dbg!(buffer_len);
            Err(
                DFGHError::ImportBufferError(line_number, buffer_len, r_e, e_string)
            )
        },
        _ => Err(DFGHError::ImportUnknownError),
    }
}

pub fn error_window(state: &mut DFGraphicsHelper, ctx: &Context) {
    egui::Window::new("Error Window")
        .collapsible(false)
        .constrain(true)
        .title_bar(true)
        .default_size([600.0, 300.0])
        .show(ctx, |ui| {

        egui::ScrollArea::both()
            .show(ui, |ui| {
                ui.add(egui::Label::new(egui::RichText::new(
                    state.exception.to_string())
                    .monospace())
                    .wrap(true)
                );
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
                        DFGHError::ImageError(..) => {
                            state.undo();
                            state.exception = DFGHError::None;
                        },
                        DFGHError::IndexError => {
                            state.main_window = MainWindow::DefaultMenu;
                            state.indices = [0, 0, 0, 0, 0, 0, 0, 0 as usize].into();
                            state.exception = DFGHError::None;
                        },
                        DFGHError::None => {},
                        _ => {state.exception = DFGHError::None;}
                    }
                }
            });
        });
    });
}