use egui::{Context, Key, KeyboardShortcut, Modifiers, Sense, Stroke, TextureHandle, TextureOptions, Ui};
use egui_plot::{GridInput, GridMark, Plot, PlotImage, PlotPoint, Polygon};
// use convert_case::{Case, Casing};
use rfd;
use std::path::PathBuf;
use std::path;

use crate::PADDING;
use super::error;
use crate::{RAW, Menu, Graphics, TilePageFile, TilePage, GraphicsFile, 
    Creature, LayerSet, LayerGroup, Layer, SimpleLayer, Condition};//, State, Caste};
use error::{DFGHError, Result, error_window};

#[derive(Debug, Default, Clone, Copy)]
pub enum MainWindow {
    #[default]
    DefaultMenu,
    TilePageFileDefaultMenu,
    TilePageFileMenu,
    TilePageMenu,
    CreatureDefaultMenu,
    GraphicsFileMenu,
    CreatureMenu,
    LayerGroupMenu,
    LayerSetMenu,
    LayerMenu,
    ConditionMenu,
    ReferenceMenu,
}

#[derive(Debug, Default, Clone)]
enum ContextData {
    #[default]
    None,
    TilePageFile(TilePageFile),
    TilePage(TilePage),
    GraphicsFile(GraphicsFile),
    Creature(Creature),
    LayerSet(LayerSet),
    LayerGroup(LayerGroup),
    Layer(Layer),
    SimpleLayer(SimpleLayer),
    Condition(Condition),
}
impl From<TilePageFile> for ContextData {
    fn from(value: TilePageFile) -> Self {
        ContextData::TilePageFile(value)
    }
}
impl From<TilePage> for ContextData {
    fn from(value: TilePage) -> Self {
        ContextData::TilePage(value)
    }
}
impl From<GraphicsFile> for ContextData {
    fn from(value: GraphicsFile) -> Self {
        ContextData::GraphicsFile(value)
    }
}
impl From<Creature> for ContextData {
    fn from(value: Creature) -> Self {
        ContextData::Creature(value)
    }
}
impl From<LayerSet> for ContextData {
    fn from(value: LayerSet) -> Self {
        ContextData::LayerSet(value)
    }
}
impl From<LayerGroup> for ContextData {
    fn from(value: LayerGroup) -> Self {
        ContextData::LayerGroup(value)
    }
}
impl From<Layer> for ContextData {
    fn from(value: Layer) -> Self {
        ContextData::Layer(value)
    }
}
impl From<SimpleLayer> for ContextData {
    fn from(value: SimpleLayer) -> Self {
        ContextData::SimpleLayer(value)
    }
}
impl From<Condition> for ContextData {
    fn from(value: Condition) -> Self {
        ContextData::Condition(value)
    }
}
impl From<MainWindow> for ContextData {
    fn from(main_window: MainWindow) -> Self {
        match main_window {
            MainWindow::DefaultMenu => ContextData::GraphicsFile(GraphicsFile::default()),
            MainWindow::TilePageFileDefaultMenu => ContextData::TilePageFile(TilePageFile::new()),
            MainWindow::TilePageFileMenu => ContextData::TilePageFile(TilePageFile::new()),
            MainWindow::TilePageMenu => ContextData::TilePage(TilePage::new()),
            MainWindow::CreatureDefaultMenu => ContextData::Creature(Creature::new()),
            MainWindow::GraphicsFileMenu => ContextData::GraphicsFile(GraphicsFile::default()),
            MainWindow::CreatureMenu => ContextData::Creature(Creature::new()),
            MainWindow::LayerGroupMenu => ContextData::LayerGroup(LayerGroup::new()),
            MainWindow::LayerSetMenu => ContextData::LayerSet(LayerSet::new()),
            MainWindow::LayerMenu => ContextData::Layer(Layer::new()),
            MainWindow::ConditionMenu => ContextData::Condition(Condition::Default),
            MainWindow::ReferenceMenu => ContextData::None,
        }
    }
}

#[derive(Debug, Default, Clone)]
enum PreviewZoom {
    #[default]
    None,
    All,
    Selected,
    HorizontalFit,
    VerticalFit,
}

#[derive(Debug, Default, Clone)]
enum Action {
    #[default]
    None,
    Copy(ContextData),
    Cut(ContextData),
    Paste,
    Duplicate(ContextData),
    Insert(ContextData),
    Undo,
    Redo,
    Delete(ContextData),
    Import,
    Export,
    Update,
    Zoom(PreviewZoom),
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GraphicsIndices {
    tile_page_file_index: usize,
    tile_page_index: usize,
    graphics_file_index: usize,
    // graphics_index: usize,
    // layer_set_index: usize,
    // layer_group_index: usize,
    // layer_index: usize,
    // condition_index: usize,
}
impl GraphicsIndices {
    fn new() -> GraphicsIndices {
        Self {
            tile_page_file_index: 0,
            tile_page_index: 0,
            graphics_file_index: 0,
            // graphics_index: 0,
            // layer_set_index: 0,
            // layer_group_index: 0,
            // layer_index: 0,
            // condition_index: 0,
        }
    }
}
impl From<[usize; 8]> for GraphicsIndices {
    fn from(index_array: [usize; 8]) -> Self {
        GraphicsIndices {
            tile_page_file_index:   index_array[0],
            tile_page_index:        index_array[1],
            graphics_file_index:    index_array[2],
            // graphics_index:         index_array[3],
            // layer_set_index:        index_array[4],
            // layer_group_index:      index_array[5],
            // layer_index:            index_array[6],
            // condition_index:        index_array[7]
        }
    }
}

pub struct DFGraphicsHelper {
    pub main_window: MainWindow,
    loaded_graphics: Graphics,
    pub indices: GraphicsIndices,
    path: std::path::PathBuf,
    preview: bool,
    preview_name: String,
    preview_bounds: Option<egui_plot::PlotBounds>,
    selected_region: Option<[[u32; 2]; 2]>,
    texture: Option<TextureHandle>,
    cursor_coords: Option<[u32; 2]>,
    action: Action,
    copied: ContextData,
    undo_buffer: Vec<(Graphics, GraphicsIndices)>,
    redo_buffer: Vec<(Graphics, GraphicsIndices)>,
    pub exception: DFGHError,
}
impl DFGraphicsHelper {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        Self {
            main_window: MainWindow::DefaultMenu,
            loaded_graphics: Graphics::new(),
            indices: GraphicsIndices::new(),
            path: path::PathBuf::new(),
            preview: false,
            preview_name: String::new(),
            preview_bounds: None,
            selected_region: None,
            texture: None,
            cursor_coords: None,
            action: Action::default(),
            copied: ContextData::default(),
            undo_buffer: Vec::with_capacity(1000),
            redo_buffer: Vec::with_capacity(100),
            exception: DFGHError::None,
        }
    }

    fn import(&mut self) {
        self.save_state();
        if !self.path.exists() {
            if let Some(mut path) = rfd::FileDialog::new()
                .set_title("Choose Mod Folder")
                .pick_folder() {
                
                let import_result = Graphics::import(&mut path);
                if let Ok((graphics, path)) = import_result {
                    (self.loaded_graphics, self.path) = (graphics, path);
                } else {
                    self.exception = import_result.unwrap_err();
                }
            }
        } else {
            let import_result = Graphics::import(&mut self.path);
            if let Ok((graphics, path)) = import_result {
                (self.loaded_graphics, self.path) = (graphics, path);
            } else {
                self.exception = import_result.unwrap_err();
            }
        }
        self.action = Action::None;
    }

    fn export(&mut self) {
        if !self.path.exists() {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Choose Mod Folder")
                .pick_folder() {
                let export_result = self.loaded_graphics.export(&path);
                if export_result.is_err() {
                    self.exception = export_result.unwrap_err();
                };
            }
        } else {
            let export_result = self.loaded_graphics.export(&self.path);
            if export_result.is_err() {
                self.exception = export_result.unwrap_err();
            };
        }
        self.action = Action::None;
    }

    pub fn undo(&mut self) {
        if let Some(pop) = self.undo_buffer.pop() {
            if self.redo_buffer.len() == self.redo_buffer.capacity() {
                self.redo_buffer.remove(0);//checked
            }
            self.redo_buffer.push((self.loaded_graphics.clone(), self.indices));

            (self.loaded_graphics, self.indices) = pop;
        }

        self.action = Action::None;
    }

    fn redo(&mut self) {
        if let Some(pop) = self.redo_buffer.pop() {
            if self.undo_buffer.len() == self.undo_buffer.capacity() {
                self.undo_buffer.remove(0);//checked
            }
            self.undo_buffer.push((self.loaded_graphics.clone(), self.indices));

            (self.loaded_graphics, self.indices) = pop;
        }

        self.action = Action::None;
    }

    fn context(ui: &mut Ui, selected: ContextData) -> Action {
        let action;
        if ui.button("Copy").clicked() {
            ui.close_menu();
            action = Action::Copy(selected);
        } else if ui.button("Cut").clicked() {
            ui.close_menu();
            action = Action::Cut(selected);
        } else if ui.button("Paste").clicked() {
            ui.close_menu();
            action = Action::Paste;
        } else if ui.button("Duplicate").clicked() {
            ui.close_menu();
            action = Action::Duplicate(selected);
        } else {
            let mut inner_action = Action::None;
            ui.menu_button("Insert..", |ui| {
                match selected {
                    ContextData::TilePageFile(_) | ContextData::TilePage(_) => {
                        if ui.button("Tile Page File").clicked() {
                            ui.close_menu();
                            let data = ContextData::from(TilePageFile::new());
                            inner_action = Action::Insert(data);
                        } else if ui.button("Tile Page").clicked() {
                            ui.close_menu();
                            let data = ContextData::from(TilePage::new());
                            inner_action = Action::Insert(data);
                        }
                    },
                    ContextData::GraphicsFile(_) |
                    ContextData::Creature(_) |
                    ContextData::LayerSet(_) |
                    ContextData::LayerGroup(_) |
                    ContextData::Layer(_) |
                    ContextData::SimpleLayer(_) |
                    ContextData::Condition(_) => {
                        if ui.button("Graphics File").clicked() {
                            ui.close_menu();
                            let data = ContextData::from(GraphicsFile::new());
                            inner_action = Action::Insert(data);
                        } else if ui.button("Creature").clicked() {
                            ui.close_menu();
                            let data = ContextData::from(Creature::new());
                            inner_action = Action::Insert(data);
                        } else if ui.button("Simple Layer").clicked() {
                            ui.close_menu();
                            let data = ContextData::from(SimpleLayer::new());
                            inner_action = Action::Insert(data);
                        } else if ui.button("Layer Set").clicked() {
                            ui.close_menu();
                            let data = ContextData::from(LayerSet::new());
                            inner_action = Action::Insert(data);
                        } else if ui.button("Layer Group").clicked() {
                            ui.close_menu();
                            let data = ContextData::from(LayerGroup::new());
                            inner_action = Action::Insert(data);
                        } else if ui.button("Layer").clicked() {
                            ui.close_menu();
                            let data = ContextData::from(Layer::new());
                            inner_action = Action::Insert(data);
                        } else if ui.button("Condition").clicked() {
                            ui.close_menu();
                            let data = ContextData::from(Condition::default());
                            inner_action = Action::Insert(data);
                        }
                    },
                    ContextData::None => {inner_action = Action::None;},
                }
            });
            action = inner_action;
        }
        return action
    }

    fn save_state(&mut self) {
        self.undo_buffer.push((self.loaded_graphics.clone(), self.indices));

        if !self.redo_buffer.is_empty() {
            self.redo_buffer.clear();
        }
    }

    fn copy(&mut self, selected: ContextData) {
        self.copied = ContextData::from(selected.clone());
        self.action = Action::None;
    }

    fn cut(&mut self, selected: ContextData) -> Result<()> {
        self.save_state();

        self.copied = ContextData::from(selected.clone());

        self.delete(selected)?;

        Ok(())
    }

    fn paste(&mut self) -> Result<()> {
        let data = self.copied.clone();
        self.insert(data)?;

        Ok(())
    }

    fn insert(&mut self, data: ContextData) -> Result<()> {
        self.save_state();

        let graphics = &mut self.loaded_graphics;
        let indices = &mut self.indices;

        match data {
            ContextData::TilePageFile(tile_page_file) => {
                let tps = &mut graphics.tile_page_files;
                if indices.tile_page_file_index < tps.len() {
                    tps.insert(indices.tile_page_file_index, tile_page_file.clone());//checked
                    self.main_window = MainWindow::TilePageFileMenu;
                }
            },
            ContextData::TilePage(tile_page) => {
                let ts = &mut graphics.tile_page_files
                    .get_mut(indices.tile_page_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .tile_pages;
                if indices.tile_page_index < ts.len() {
                    ts.insert(indices.tile_page_index, tile_page.clone());//checked
                    self.main_window = MainWindow::TilePageMenu;
                }
            },
            ContextData::GraphicsFile(graphics_file) => {
                let gfs = &mut graphics.graphics_files;
                if indices.graphics_file_index < gfs.len() {
                    gfs.insert(indices.graphics_file_index, graphics_file.clone());//checked
                    self.main_window = MainWindow::GraphicsFileMenu;
                }
            },
            ContextData::Creature(_creature) => {
                // let cs = &mut graphics.graphics_files
                //     .get_mut(indices.graphics_file_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .creatures;
                // if indices.graphics_index < cs.len() {
                //     cs.insert(indices.graphics_index, creature.clone());//checked
                //     self.main_window = MainWindow::CreatureMenu;
                // }
            },
            ContextData::LayerSet(_layer_set) => {
                // let lss = &mut graphics.graphics_files
                //     .get_mut(indices.graphics_file_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .creatures
                //     .get_mut(indices.graphics_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .graphics_type;
                // if indices.layer_set_index < lss.len() {
                //     lss.insert(indices.layer_set_index, layer_set.clone());//checked
                //     self.main_window = MainWindow::LayerSetMenu;
                // }
            },
            ContextData::LayerGroup(_layer_group) => {
                // if let LayerSet::Layered(_, layer_groups) = graphics.graphics_files
                //     .get_mut(indices.graphics_file_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .creatures
                //     .get_mut(indices.graphics_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .graphics_type
                //     .get_mut(indices.layer_set_index)
                //     .ok_or(DFGHError::IndexError)? {
                //     if indices.layer_group_index < layer_groups.len() {
                //         layer_groups.insert(indices.layer_group_index, layer_group.clone());//checked
                //         self.main_window = MainWindow::LayerGroupMenu;
                //     }
                // }
            },
            ContextData::SimpleLayer(_simple_layer) => {
                // if let LayerSet::Simple(simple_layers) |
                //     LayerSet::Statue(simple_layers) = 
                //     graphics.graphics_files
                //     .get_mut(indices.graphics_file_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .creatures
                //     .get_mut(indices.graphics_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .graphics_type
                //     .get_mut(indices.layer_set_index)
                //     .ok_or(DFGHError::IndexError)? {
                //     if indices.layer_index < simple_layers.len() {
                //         simple_layers.insert(indices.layer_index, simple_layer.clone());//checked
                //         self.main_window = MainWindow::LayerMenu;
                //     }
                // }
            },
            ContextData::Layer(_layer) => {
                // if let LayerSet::Layered(_, layer_groups) = graphics.graphics_files
                //     .get_mut(indices.graphics_file_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .creatures
                //     .get_mut(indices.graphics_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .graphics_type
                //     .get_mut(indices.layer_set_index)
                //     .ok_or(DFGHError::IndexError)? {
                //     let ls = &mut layer_groups
                //         .get_mut(indices.layer_group_index)
                //         .ok_or(DFGHError::IndexError)?
                //         .layers;
                //     if indices.layer_index < ls.len() {
                //         ls.insert(indices.layer_index, layer.clone());//checked
                //         self.main_window = MainWindow::LayerMenu;
                //     }
                // }
            },
            ContextData::Condition(_condition) => {
                // if let LayerSet::Layered(_, layer_groups) = graphics.graphics_files
                //     .get_mut(indices.graphics_file_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .creatures
                //     .get_mut(indices.graphics_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .graphics_type
                //     .get_mut(indices.layer_set_index)
                //     .ok_or(DFGHError::IndexError)? {
                //     let cs = &mut layer_groups
                //         .get_mut(indices.layer_group_index)
                //         .ok_or(DFGHError::IndexError)?
                //         .layers
                //         .get_mut(indices.layer_index)
                //         .ok_or(DFGHError::IndexError)?
                //         .conditions;
                //     if indices.condition_index < cs.len() {
                //         cs.insert(indices.condition_index, condition.clone());//checked
                //         self.main_window = MainWindow::ConditionMenu;
                //     }
                // }
            },
            ContextData::None => {},
        }
        self.action = Action::None;
        Ok(())
    }

    fn delete(&mut self, selected: ContextData) -> Result<()> {
        self.save_state();
        
        let graphics = &mut self.loaded_graphics;
        let indices = &mut self.indices;

        match selected {
            ContextData::TilePageFile(_) => {
                let tps = &mut graphics.tile_page_files;
                if indices.tile_page_file_index < tps.len() {
                    tps.remove(indices.tile_page_file_index);//checked
                    if indices.tile_page_file_index >=1 {
                        indices.tile_page_file_index -= 1;
                    } else {
                        self.main_window = MainWindow::TilePageFileDefaultMenu;
                    }
                }
            },
            ContextData::TilePage(_) => {
                let ts = &mut graphics.tile_page_files
                    .get_mut(indices.tile_page_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .tile_pages;
                if indices.tile_page_index < ts.len() {
                    ts.remove(indices.tile_page_index);//checked
                    if indices.tile_page_index >=1 {
                        indices.tile_page_index -= 1;
                    } else {
                        self.main_window = MainWindow::TilePageFileMenu;
                    }
                }
            },
            ContextData::GraphicsFile(_) => {
                let cfs = &mut graphics.graphics_files;
                if indices.graphics_file_index < cfs.len() {
                    cfs.remove(indices.graphics_file_index);//checked
                    if indices.graphics_file_index >=1 {
                        indices.graphics_file_index -= 1;
                    } else {
                        self.main_window = MainWindow::CreatureDefaultMenu;
                    }
                }
            },
            ContextData::Creature(_) => {
                // let cs = &mut graphics.graphics_files
                //     .get_mut(indices.graphics_file_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .creatures;
                // if indices.graphics_index < cs.len() {
                //     cs.remove(indices.graphics_index);//checked
                //     if indices.graphics_index >=1 {
                //         indices.graphics_index -= 1;
                //     } else {
                //         self.main_window = MainWindow::GraphicsFileMenu;
                //     }
                // }
            },
            ContextData::LayerSet(_) => {
                // let lss = &mut graphics.graphics_files
                //     .get_mut(indices.graphics_file_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .creatures
                //     .get_mut(indices.graphics_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .graphics_type;
                // if indices.layer_set_index < lss.len() {
                //     lss.remove(indices.layer_set_index);//checked
                //     if indices.layer_set_index >=1 {
                //         indices.layer_set_index -= 1;
                //     } else {
                //         self.main_window = MainWindow::CreatureMenu;
                //     }
                // }
            },
            ContextData::LayerGroup(_) => {
                // if let LayerSet::Layered(_, layer_groups) = graphics.graphics_files
                //     .get_mut(indices.graphics_file_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .creatures
                //     .get_mut(indices.graphics_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .graphics_type
                //     .get_mut(indices.layer_set_index)
                //     .ok_or(DFGHError::IndexError)? {
                //     if indices.layer_group_index < layer_groups.len() {
                //         layer_groups.remove(indices.layer_group_index);//checked
                //         if indices.layer_group_index >=1 {
                //             indices.layer_group_index -= 1;
                //         } else {
                //             self.main_window = MainWindow::LayerSetMenu;
                //         }
                //     }
                // }
            },
            ContextData::SimpleLayer(_) => {
                // if let LayerSet::Simple(simple_layers) |
                //     LayerSet::Statue(simple_layers) = 
                //     graphics.graphics_files
                //     .get_mut(indices.graphics_file_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .creatures
                //     .get_mut(indices.graphics_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .graphics_type
                //     .get_mut(indices.layer_set_index)
                //     .ok_or(DFGHError::IndexError)? {
                //     if indices.layer_index < simple_layers.len() {
                //         simple_layers.remove(indices.layer_index);//checked
                //         if indices.layer_index >=1 {
                //             indices.layer_index -= 1;
                //         } else {
                //             self.main_window = MainWindow::CreatureMenu;
                //         }
                //     }
                // }
            },
            ContextData::Layer(_) => {
                // if let LayerSet::Layered(_, layer_groups) = graphics.graphics_files
                //     .get_mut(indices.graphics_file_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .creatures
                //     .get_mut(indices.graphics_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .graphics_type
                //     .get_mut(indices.layer_set_index)
                //     .ok_or(DFGHError::IndexError)? {
                //     let ls = &mut layer_groups
                //         .get_mut(indices.layer_group_index)
                //         .ok_or(DFGHError::IndexError)?
                //         .layers;
                //     if indices.layer_index < ls.len() {
                //         ls.remove(indices.layer_index);//checked
                //         if indices.layer_index >=1 {
                //             indices.layer_index -= 1;
                //         } else {
                //             self.main_window = MainWindow::LayerGroupMenu;
                //         }
                //     }
                // }
            },
            ContextData::Condition(_) => {
                // if let LayerSet::Layered(_, layer_groups) = graphics.graphics_files
                //     .get_mut(indices.graphics_file_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .creatures
                //     .get_mut(indices.graphics_index)
                //     .ok_or(DFGHError::IndexError)?
                //     .graphics_type
                //     .get_mut(indices.layer_set_index)
                //     .ok_or(DFGHError::IndexError)? {
                //     let cs = &mut layer_groups
                //         .get_mut(indices.layer_group_index)
                //         .ok_or(DFGHError::IndexError)?
                //         .layers
                //         .get_mut(indices.layer_index)
                //         .ok_or(DFGHError::IndexError)?
                //         .conditions;
                //     if indices.condition_index < cs.len() {
                //         cs.remove(indices.condition_index);//checked
                //         if indices.condition_index >=1 {
                //             indices.condition_index -= 1;
                //         } else {
                //             self.main_window = MainWindow::LayerMenu;
                //         }
                //     }
                // }
            },
            ContextData::None => {},
        }
        self.action = Action::None;
        Ok(())
    }

    fn update(&mut self) {
        self.texture = None;
        self.preview = false;
        self.preview_name = "".to_string();
        self.loaded_graphics.update_shared(&self.path)
    }

    fn zoom(&mut self, zoom: PreviewZoom) {
        let mut min: [f64; 2] = [-2.0, -66.0];
        let mut max: [f64; 2] = [98.0, 2.0];

        if let Some(texture) = &self.texture {
            let size = [texture.size_vec2().x as f64, texture.size_vec2().y as f64];

            match zoom {
                PreviewZoom::All => {
                    min = [-0.02*size[0], -1.02*size[1]];
                    max = [1.02*size[0], 0.02*size[1]];
                },
                PreviewZoom::Selected => {
                    
                },
                PreviewZoom::HorizontalFit => {
                    min = [-0.02*size[0], -1.02*size[0]];
                    max = [1.02*size[0], 0.02*size[0]];
                },
                PreviewZoom::VerticalFit => {
                    min = [-0.02*size[1], -1.02*size[1]];
                    max = [1.02*size[1], 0.02*size[1]];
                },
                PreviewZoom::None => {},
            }
            self.preview_bounds = Some(egui_plot::PlotBounds::from_min_max(min, max));
        }
        
        self.action = Action::None;
    }

    fn main_tree(&mut self, ui: &mut Ui, ctx: &Context) {
        let graphics = &mut self.loaded_graphics;

        if ui.add(egui::Label::new("Tile Page Files")
            .wrap(false)
            .sense(Sense::click()))
            .clicked()
        {
            self.main_window = MainWindow::TilePageFileDefaultMenu;
        };
        for (i_tile_page_file, tile_page_file) in graphics.tile_page_files.iter_mut().enumerate() {
            let id_t = ui.make_persistent_id(
                format!("tile_page_file{}", i_tile_page_file)
            );
            egui::collapsing_header::CollapsingState::load_with_default_open(
                ctx,
                id_t,
                true,
            )
            .show_header(ui, |ui| {
                let tile_page_file_response = ui.add(egui::Label::new(
                    format!("{}", &tile_page_file.name))
                    .sense(Sense::click()));
                if tile_page_file_response.clicked() {
                    self.indices = [i_tile_page_file, 0, 0, 0, 0, 0, 0, 0].into();
                    self.main_window = MainWindow::TilePageFileMenu;
                }
                tile_page_file_response.context_menu(|ui| {
                    self.indices = [i_tile_page_file, 0, 0, 0, 0, 0, 0, 0].into();
                    self.action = Self::context(ui, ContextData::from(tile_page_file.clone()));
                });
            })
            .body(|ui| {
                for (i_tile_page, tile_page) in tile_page_file.tile_pages.iter_mut().enumerate() {
                    let tile_page_response = ui.add(egui::Label::new(
                        format!("{}", &tile_page.name))
                        .wrap(false)
                        .sense(Sense::click()));
                    if tile_page_response.clicked() {
                        self.indices = [i_tile_page_file, i_tile_page, 0, 0, 0, 0, 0, 0].into();
                        self.main_window = MainWindow::TilePageMenu;
                    }
                    tile_page_response.context_menu(|ui| {
                        self.indices = [i_tile_page_file, i_tile_page, 0, 0, 0, 0, 0, 0].into();
                        self.action = Self::context(ui, ContextData::from(tile_page.clone()));
                    });
                }
            });
        }

        ui.separator();
        if ui.add(egui::Label::new("Creature Graphics")
            .wrap(false)
            .sense(Sense::click()))
            .clicked()
        {
            self.main_window = MainWindow::CreatureDefaultMenu;
        };
        // for (i_file, graphics_file) in self.loaded_graphics.graphics_files.iter_mut().enumerate() {
        //     let id_cf = ui.make_persistent_id(
        //         format!("graphics_file{}",
        //         i_file)
        //     );
        //     egui::collapsing_header::CollapsingState::load_with_default_open(
        //         ctx,
        //         id_cf,
        //         true,
        //     )
        //     .show_header(ui, |ui| {
        //         let graphics_file_response = ui.add(egui::Label::new(
        //             format!("{}", &graphics_file.name))
        //             .sense(Sense::click()));
        //         if graphics_file_response.clicked() {
        //             self.indices = [0, 0, i_file, 0, 0, 0, 0, 0].into();
        //             self.main_window = MainWindow::GraphicsFileMenu;
        //         }
        //         graphics_file_response.context_menu(|ui| {
        //             self.indices = [0, 0, i_file, 0, 0, 0, 0, 0].into();
        //             self.action = Self::context(ui, ContextData::from(graphics_file.clone()));
        //         });
        //     })
        //     .body(|ui| {
        //         for (i_graphics, graphics) in graphics_file.creatures.iter_mut().enumerate() {
        //             let id_c = ui.make_persistent_id(
        //                 format!("graphics{}{}",
        //                 i_file, i_graphics)
        //             );
        //             egui::collapsing_header::CollapsingState::load_with_default_open(
        //                 ctx,
        //                 id_c,
        //                 true,
        //             )
        //             .show_header(ui, |ui| {
        //                 let graphics_response = ui.add(egui::Label::new(
        //                     format!("{}", &graphics.name))
        //                     .sense(Sense::click()));
        //                 if graphics_response.clicked() {
        //                     self.indices = [0, 0, i_file, i_graphics, 0, 0, 0, 0].into();
        //                     self.main_window = MainWindow::CreatureMenu;
        //                 }
        //                 graphics_response.context_menu(|ui| {
        //                     self.indices = [0, 0, i_file, i_graphics, 0, 0, 0, 0].into();
        //                     self.action = Self::context(ui, ContextData::from(graphics.clone()));
        //                 });
        //             })
        //             .body(|ui| {
        //                 for (i_layer_set, layer_set) in graphics.graphics_type.iter_mut().enumerate() {
        //                     match layer_set {
        //                         LayerSet::Empty => {
        //                             let layerset_response = ui.add(egui::Label::new("(empty)")
        //                                 .sense(Sense::click()));
        //                             if layerset_response.clicked() {
        //                                 self.indices = [0, 0, i_file, i_graphics, i_layer_set, 0, 0, 0].into();
        //                                 self.main_window = MainWindow::LayerSetMenu;
        //                             }
        //                             layerset_response.context_menu(|ui| {
        //                                 self.indices = [0, 0, i_file, i_graphics, i_layer_set, 0, 0, 0].into();
        //                                 self.action = Self::context(ui, ContextData::from(layer_set.clone()));
        //                             });
        //                         },
        //                         LayerSet::Layered(state, layer_groups) => {
        //                             let id_ls = ui.make_persistent_id(
        //                                 format!("layer_set{}{}{}",
        //                                 i_file, i_graphics, i_layer_set)
        //                             );
        //                             egui::collapsing_header::CollapsingState::load_with_default_open(ctx,
        //                                 id_ls,
        //                                 true)
        //                                 .show_header(ui, |ui|
        //                                 {
        //                                     let layerset_response = ui.add(egui::Label::new(
        //                                         format!("{}", state.name()))
        //                                         .sense(Sense::click()));
        //                                     if layerset_response.clicked() {
        //                                         self.indices = [0, 0, i_file, i_graphics, i_layer_set, 0, 0, 0].into();
        //                                         self.main_window = MainWindow::LayerSetMenu;
        //                                     }
        //                                     layerset_response.context_menu(|ui| {
        //                                         self.indices = [0, 0, i_file, i_graphics, i_layer_set, 0, 0, 0].into();
        //                                         self.action = Self::context(ui, ContextData::from(LayerSet::Layered(state.clone(), layer_groups.clone())));
        //                                     });
        //                             })
        //                                 .body(|ui|
        //                                 {
        //                                 for (i_layer_group, layer_group) in layer_groups.iter_mut().enumerate() {
        //                                     let id_lg = ui.make_persistent_id(
        //                                         format!("layer_group{}{}{}{}",
        //                                         i_file, i_graphics, i_layer_set, i_layer_group)
        //                                     );
        //                                     egui::collapsing_header::CollapsingState::load_with_default_open(ctx,
        //                                         id_lg,
        //                                         false)
        //                                         .show_header(ui, |ui|
        //                                         {
        //                                             let layergroup_response = ui.add(egui::Label::new(
        //                                                 format!("{}", &layer_group.name))
        //                                                 .sense(Sense::click()));
        //                                             if layergroup_response.clicked() {
        //                                                 self.indices = [0, 0, i_file, i_graphics, i_layer_set, i_layer_group, 0, 0].into();
        //                                                 self.main_window = MainWindow::LayerGroupMenu;
        //                                             }
        //                                             layergroup_response.context_menu(|ui| {
        //                                                 self.indices = [0, 0, i_file, i_graphics, i_layer_set, i_layer_group, 0, 0].into();
        //                                                 self.action = Self::context(ui, ContextData::from(layer_group.clone()));
        //                                             });
        //                                         })
        //                                         .body(|ui|
        //                                         {
        //                                         for (i_layer, layer) in layer_group.layers.iter_mut().enumerate() {
        //                                             let id_l = ui.make_persistent_id(
        //                                                 format!("layer{}{}{}{}{}",
        //                                                 i_file, i_graphics, i_layer_set, i_layer_group, i_layer)
        //                                             );
        //                                             egui::collapsing_header::CollapsingState::load_with_default_open(ctx,
        //                                                 id_l,
        //                                                 false)
        //                                                 .show_header(ui, |ui|
        //                                                 {
        //                                                     let layer_response = ui.add(egui::Label::new(
        //                                                         format!("{}", &layer.name))
        //                                                         .sense(Sense::click()));
        //                                                     if layer_response.clicked() {
        //                                                         self.indices = [0, 0, i_file, i_graphics, i_layer_set, i_layer_group, i_layer, 0].into();
        //                                                         self.main_window = MainWindow::LayerMenu;
        //                                                     }
        //                                                     layer_response.context_menu(|ui| {
        //                                                         self.indices = [0, 0, i_file, i_graphics, i_layer_set, i_layer_group, i_layer, 0].into();
        //                                                         self.action = Self::context(ui, ContextData::from(layer.clone()));
        //                                                     });
        //                                                 })
        //                                                 .body(|ui|
        //                                                 {
        //                                                 for (i_condition, condition) in layer.conditions.iter_mut().enumerate() {
        //                                                     let condition_response = ui.add(egui::Label::new(
        //                                                         format!("\t{}", condition.name()))
        //                                                         .wrap(false)
        //                                                         .sense(Sense::click()));
        //                                                     if condition_response.clicked() {
        //                                                         self.indices = [0, 0, i_file, i_graphics, i_layer_set, i_layer_group, i_layer, i_condition].into();
        //                                                         self.main_window = MainWindow::ConditionMenu;
        //                                                     }
        //                                                     condition_response.context_menu(|ui| {
        //                                                         self.indices = [0, 0, i_file, i_graphics, i_layer_set, i_layer_group, i_layer, i_condition].into();
        //                                                         self.action = Self::context(ui, ContextData::from(condition.clone()));
        //                                                     });
        //                                                 }
        //                                             });
        //                                         }
        //                                     });
        //                                 }
        //                             });
        //                         },
        //                         LayerSet::Simple(simple_layers) => {
        //                             for (i_layer, simple_layer) in simple_layers.iter_mut().enumerate() {
        //                                 let condition_response = ui.add(egui::Label::new(
        //                                 if let Some(sub_state) = &simple_layer.sub_state {
        //                                         format!("\t{} + {}",
        //                                         simple_layer.state.name(),
        //                                         sub_state.name())
        //                                     } else {
        //                                         format!("\t{}",
        //                                         simple_layer.state.name())
        //                                     })
        //                                     .wrap(false)
        //                                     .sense(Sense::click()));
        //                                 if condition_response.clicked() {
        //                                     self.indices = [0, 0, i_file, i_graphics, i_layer_set, 0, i_layer, 0].into();
        //                                     self.main_window = MainWindow::LayerMenu;
        //                                 }
        //                                 condition_response.context_menu(|ui| {
        //                                     self.indices = [0, 0, i_file, i_graphics, i_layer_set, 0, i_layer, 0].into();
        //                                     self.action = Self::context(ui, ContextData::from(simple_layer.clone()));
        //                                 });
        //                             }
        //                         },
        //                         LayerSet::Statue(simple_layers) => {
        //                             for (i_layer, simple_layer) in simple_layers.iter_mut().enumerate() {
        //                                 let condition_response = ui.add(egui::Label::new(
        //                                 if let Some(sub_state) = &simple_layer.sub_state {
        //                                         format!("\t{} + {}",
        //                                         simple_layer.state.name(),
        //                                         sub_state.name())
        //                                     } else {
        //                                         format!("\t{}",
        //                                         simple_layer.state.name())
        //                                     })
        //                                     .wrap(false)
        //                                     .sense(Sense::click()));
        //                                 if condition_response.clicked() {
        //                                     self.indices = [0, 0, i_file, i_graphics, i_layer_set, 0, i_layer, 0].into();
        //                                     self.main_window = MainWindow::LayerMenu;
        //                                 }
        //                                 condition_response.context_menu(|ui| {
        //                                     self.indices = [0, 0, i_file, i_graphics, i_layer_set, 0, i_layer, 0].into();
        //                                     self.action = Self::context(ui, ContextData::from(simple_layer.clone()));
        //                                 });
        //                             }
        //                         },
        //                     }
        //                 }
        //             });
        //         }
        //     });
        // }
    }

    fn default_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.label("Welcome!");
        ui.separator();

        ui.add_space(PADDING);
        ui.hyperlink_to(
            "DF Graphics Helper on GitHub",
            "https://github.com/BarelyCreative/DF-graphics-helper/tree/main",
        );

        self.preview = false;
        self.preview_name = String::new();

        Ok(())
    }

    fn tile_page_file_default_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.label("Tile Page File Menu");
        ui.separator();

        if ui.small_button("New Tile Page File").clicked() {
            self.action = Action::Insert(ContextData::TilePageFile(TilePageFile::new()));
        }

        self.preview = false;
        self.preview_name = String::new();

        Ok(())
    }

    fn tile_page_file_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Tile Page File Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::TilePageFile(TilePageFile::new()));
            }
        });

        let indices = &mut self.indices;

        if self.loaded_graphics.tile_page_files.is_empty() {
            self.action = Action::Insert(ContextData::TilePageFile(TilePageFile::new()));
        } else {
            let tile_file = self
                .loaded_graphics
                .tile_page_files
                .get_mut(indices.tile_page_file_index)
                .ok_or(DFGHError::IndexError)?;

            ui.separator();
            ui.text_edit_singleline(&mut tile_file.name);
            ui.add_space(PADDING);

            if ui.button("New Tile Page").clicked() {
                self.action = Action::Insert(ContextData::TilePage(TilePage::new()));
            }
        }

        self.preview = false;
        self.preview_name = String::new();
        
        Ok(())
    }

    fn tile_page_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Tile Page Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::TilePage(TilePage::new()));
            }
        });
        
        let indices = &mut self.indices;

        let tile_pages = &mut self
            .loaded_graphics
            .tile_page_files
            .get_mut(indices.tile_page_file_index)
            .ok_or(DFGHError::IndexError)?
            .tile_pages;

        if tile_pages.is_empty() {
            if ui.small_button("Create Tile Page").clicked() {
                self.action = Action::Insert(ContextData::TilePage(TilePage::new()));
            }
        } else {
            let tile_page = tile_pages
                .get_mut(indices.tile_page_index)
                .ok_or(DFGHError::IndexError)?;

            let shared = &mut self.loaded_graphics.shared;

            tile_page.menu(ui, shared);

            self.preview = true;
            self.preview_name = tile_page.name.clone();
        }
        Ok(())
    }

    fn graphics_default_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.label("Graphics File Menu");
        ui.separator();

        if ui.small_button("New Graphics File").clicked() {
            self.action = Action::Insert(ContextData::GraphicsFile(GraphicsFile::new()));
        }

        self.preview = false;
        self.preview_name = String::new();

        Ok(())
    }

    fn graphics_file_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Graphics File Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::GraphicsFile(GraphicsFile::new()));
            }
        });
        
        // let indices = &mut self.indices;

        if self.loaded_graphics.graphics_files.is_empty() {
            self.main_window = MainWindow::CreatureDefaultMenu;
        } else {
            // let graphics_file = self
            //     .loaded_graphics
            //     .graphics_files
            //     .get_mut(indices.graphics_file_index)
            //     .ok_or(DFGHError::IndexError)?;

            ui.separator();
            // ui.text_edit_singleline(&mut graphics_file.name);
            ui.add_space(PADDING);

            if ui.button("New Graphics").clicked() {
                self.action = Action::Insert(ContextData::Creature(Creature::new()));
            }
        }

        self.preview = false;
        self.preview_name = String::new();
        
        Ok(())
    }

    fn graphics_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Creature Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::Creature(Creature::new()));
            }
        });

        // let indices = &mut self.indices;

        // let graphics = &mut self
        //     .loaded_graphics
        //     .graphics_files
        //     .get_mut(indices.graphics_file_index)
        //     .ok_or(DFGHError::IndexError)?
        //     .creatures;
        
        // if graphics.is_empty() {
        //     if ui.small_button("Create Graphics").clicked() {
        //         self.action = Action::Insert(ContextData::Creature(Creature::new()));
        //     }
        // } else {
        //     let creature = graphics
        //         .get_mut(indices.graphics_index)
        //         .ok_or(DFGHError::IndexError)?;

        //     creature.creature_menu(ui);
        // }
        
        self.preview = false;
        self.preview_name = String::new();
        
        Ok(())
    }

    fn layer_set_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Layer Set Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::LayerSet(LayerSet::new()));
            }
        });

        // let indices = &mut self.indices;

        // let layer_sets = &mut self
        //     .loaded_graphics
        //     .graphics_files
        //     .get_mut(indices.graphics_file_index)
        //     .ok_or(DFGHError::IndexError)?
        //     .creatures
        //     .get_mut(indices.graphics_index)
        //     .ok_or(DFGHError::IndexError)?
        //     .graphics_type;
        
        // if layer_sets.is_empty() {
        //     if ui.small_button("Create Layer Set").clicked() {
        //         self.action = Action::Insert(ContextData::LayerSet(LayerSet::default()));
        //     }
        // } else {
        //     let layer_set = layer_sets
        //         .get_mut(indices.layer_set_index)
        //         .ok_or(DFGHError::IndexError)?;

        //     layer_set.layer_set_menu(ui);
        // }
        Ok(())
    }

    fn layer_group_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Layer Group Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::LayerGroup(LayerGroup::new()));
            }
        });
        
        // let indices = &mut self.indices;

        // let graphics_type = self
        //     .loaded_graphics
        //     .graphics_files
        //     .get_mut(indices.graphics_file_index)
        //     .ok_or(DFGHError::IndexError)?
        //     .creatures
        //     .get_mut(indices.graphics_index)
        //     .ok_or(DFGHError::IndexError)?
        //     .graphics_type
        //     .get_mut(indices.layer_set_index)
        //     .ok_or(DFGHError::IndexError)?;

        // if let LayerSet::Layered(_, layer_groups) = graphics_type {
        //     if layer_groups.is_empty() {
        //         if ui.small_button("Create Layer Group").clicked() {
        //             self.action = Action::Insert(ContextData::LayerGroup(LayerGroup::new()));
        //         }
        //     } else {
        //         let layer_group = layer_groups
        //             .get_mut(indices.layer_group_index)
        //             .ok_or(DFGHError::IndexError)?;

        //         layer_group.layer_group_menu(ui);
        //     }
        // }
        Ok(())
    }

    fn layer_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.label("layer menu");
        // let tile_info = self.tile_info();

        // let cursor_coords = self.cursor_coords;
        
        // let indices = &mut self.indices;

        // let layer_groups = self
        //     .loaded_graphics
        //     .graphics_files
        //     .get_mut(indices.graphics_file_index)
        //     .ok_or(DFGHError::IndexError)?
        //     .creatures
        //     .get_mut(indices.graphics_index)
        //     .ok_or(DFGHError::IndexError)?
        //     .graphics_type
        //     .get_mut(indices.layer_set_index)
        //     .ok_or(DFGHError::IndexError)?;

        // let tile_pages: Vec<&TilePage> = self
        //     .loaded_graphics
        //     .tile_page_files
        //     .iter()
        //     .flat_map(|tp| tp.tile_pages.iter())
        //     .collect();

        // match layer_groups {
        //     LayerSet::Simple(simple_layers) => {
        //         if simple_layers.is_empty() {
        //             //if there are no layers defined show create layer button only
        //             ui.label("Simple Layer Menu");
        //             ui.separator();
        //             if ui.small_button("Create Layer").clicked() {
        //                 self.action = Action::Insert(ContextData::SimpleLayer(SimpleLayer::new()));
        //             }
        //         } else {
        //             ui.horizontal(|ui| {
        //                 ui.label("Layer Menu");
        //                 if ui.button("Delete").clicked() {
        //                     self.action = Action::Delete(ContextData::SimpleLayer(SimpleLayer::new()));
        //                 }
        //             });

        //             let simple_layer = simple_layers
        //                 .get_mut(indices.layer_index)
        //                 .ok_or(DFGHError::IndexError)?;

        //             if let Some(coords) = cursor_coords {
        //                 simple_layer.coords = coords;
        //             }

        //             simple_layer.layer_menu(ui, tile_info);

        //             ui.add_space(PADDING);
        //             let mut file_name = String::new();
        //             for tile_page in tile_pages {
        //                 if tile_page.name.to_case(Case::UpperSnake).eq(&simple_layer.tile_name.to_case(Case::UpperSnake)) {
        //                     file_name = tile_page.file_name.clone();
        //                     break;
        //                 }
        //             }
        //             let rect = Some([simple_layer.coords, simple_layer.large_coords.unwrap_or_else(|| [0, 0])]);
        //             self.preview_image(ui, file_name, rect)?;
        //         }
        //     },
        //     LayerSet::Statue(simple_layers) => {
        //         if simple_layers.is_empty() {
        //             //if there are no layers defined show create layer button only
        //             ui.label("Statue Layer Menu");
        //             ui.separator();
        //             if ui.small_button("Create Layer").clicked() {
        //                 self.action = Action::Insert(ContextData::SimpleLayer(SimpleLayer::new()));
        //             }
        //         } else {
        //             ui.horizontal(|ui| {
        //                 ui.label("Layer Menu");
        //                 if ui.button("Delete").clicked() {
        //                     self.action = Action::Delete(ContextData::SimpleLayer(SimpleLayer::new()));
        //                 }
        //             });
                    
        //             let simple_layer = simple_layers
        //                 .get_mut(indices.layer_index)
        //                 .ok_or(DFGHError::IndexError)?;

        //             if let Some(coords) = cursor_coords {
        //                 simple_layer.coords = coords;
        //             }

        //             simple_layer.statue_layer_menu(ui, tile_info);

        //             ui.add_space(PADDING);
        //             let mut file_name = String::new();
        //             for tile_page in tile_pages {
        //                 if tile_page.name.to_case(Case::UpperSnake).eq(&simple_layer.tile_name.to_case(Case::UpperSnake)) {
        //                     file_name = tile_page.file_name.clone();
        //                     break;
        //                 }
        //             }
        //             let rect = Some([simple_layer.coords, simple_layer.large_coords.unwrap_or_else(|| [0, 0])]);
        //             self.preview_image(ui, file_name, rect)?;
        //         }
        //     },
        //     LayerSet::Layered(_, layer_groups) => {
        //         let layers = 
        //             &mut layer_groups
        //             .get_mut(indices.layer_group_index)
        //             .ok_or(DFGHError::IndexError)?
        //             .layers;

        //         if layers.is_empty() {
        //             //if there are no layers defined show create layer button only
        //             ui.label("Layer Menu");
        //             ui.separator();
        //             if ui.small_button("Create Layer").clicked() {
        //                 self.action = Action::Insert(ContextData::Layer(Layer::new()));
        //             }
        //         } else {
        //             ui.horizontal(|ui| {
        //                 ui.label("Layer Menu");
        //                 if ui.button("Delete").clicked() {
        //                     self.action = Action::Delete(ContextData::Layer(Layer::new()));
        //                 }
        //             });

        //             let layer = layers
        //                 .get_mut(indices.layer_index)
        //                 .ok_or(DFGHError::IndexError)?;

        //             if let Some(coords) = cursor_coords {
        //                 layer.coords = coords;
        //             }

        //             layer.layer_menu(ui, tile_info);

        //             ui.add_space(PADDING);
        //             let mut file_name = String::new();
        //             for tile_page in tile_pages {
        //                 if tile_page.name.to_case(Case::UpperSnake).eq(&layer.tile_name.to_case(Case::UpperSnake)) {
        //                     file_name = tile_page.file_name.clone();
        //                     break;
        //                 }
        //             }
        //             let rect = Some([layer.coords, layer.large_coords.unwrap_or_else(|| [0, 0])]);
        //             self.preview_image(ui, file_name, rect)?;
        //         }

        //     },
        //     LayerSet::Empty => {
        //         ui.horizontal(|ui| {
        //             ui.label("Empty Layer Menu");
        //             if ui.button("Delete").clicked() {
        //                 self.action = Action::Delete(ContextData::Layer(Layer::new()));
        //             }
        //         });
        //     },
        // }
        Ok(())
    }

    fn condition_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Condition Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::Condition(Condition::default()));
            }
        });

        // let tile_info = self.tile_info();
        
        // let indices = &mut self.indices;

        // let graphics_type = self
        //     .loaded_graphics
        //     .graphics_files
        //     .get_mut(indices.graphics_file_index)
        //     .ok_or(DFGHError::IndexError)?
        //     .creatures
        //     .get_mut(indices.graphics_index)
        //     .ok_or(DFGHError::IndexError)?
        //     .graphics_type
        //     .get_mut(indices.layer_set_index)
        //     .ok_or(DFGHError::IndexError)?;

        // if let LayerSet::Layered(_, layergroups) = graphics_type {
        //     let conditions = &mut layergroups
        //         .get_mut(indices.layer_group_index)
        //         .ok_or(DFGHError::IndexError)?
        //         .layers
        //         .get_mut(indices.layer_index)
        //         .ok_or(DFGHError::IndexError)?
        //         .conditions;

        //     if conditions.is_empty() {
        //         if ui.small_button("New condition").clicked() {
        //             self.action = Action::Insert(ContextData::Condition(Condition::default()));
        //         }
        //     } else {
        //         let condition = conditions
        //             .get_mut(indices.condition_index)
        //             .ok_or(DFGHError::IndexError)?;

        //         ui.separator();
    
        //         condition.condition_menu(ui, tile_info);
        //     }
        // }

        self.preview = false;
        self.preview_name = String::new();
        
        Ok(())
    }

    fn preview_image(&mut self, ui: &mut Ui) -> Result<()> {
        ui.label("Preview");
        ui.separator();

        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui: &mut Ui| {//Menu bar
                ui.menu_button("Zoom", |ui| {
                    if ui.button("All").clicked() {
                        //show all
                        self.action = Action::Zoom(PreviewZoom::All);
                        ui.close_menu();
                    }
                    if ui.button("Selected").clicked() {
                        //crop to selected rectangle
                        self.action = Action::Zoom(PreviewZoom::Selected);
                        ui.close_menu();
                    }
                    if ui.button("Fit Horizontal").clicked() {
                        //show all horizontal
                        self.action = Action::Zoom(PreviewZoom::HorizontalFit);
                        ui.close_menu();
                    }
                    if ui.button("Fit Vertical").clicked() {
                        //show all vertical
                        self.action = Action::Zoom(PreviewZoom::VerticalFit);
                        ui.close_menu();
                    }
                });
                if ui.button("Reload").clicked() {
                    self.loaded_graphics.shared.tile_page_info.remove_entry(&self.preview_name);
                    self.action = Action::Update;
                }
            });
            ui.label("Right click to set coordinates.");

            match self.draw_image(ui) {
                Ok(_) => {},
                Err(e) => {self.exception = DFGHError::from(e)}
            }
        });
        

        Ok(())
    }

    fn draw_image(&mut self, ui: &mut Ui) -> Result<()> {
        //shared formatter functions
        let label_fmt = |_s: &str, val: &PlotPoint| {
            format!(
                "{}, {}",
                (val.x / 32.0).floor(),
                (val.y / -32.0).floor()
            )
        };
        let grid_fmt = |input: GridInput| -> Vec<GridMark> {
            let mut marks = vec![];

            let (min, max) = input.bounds;
            let min = min.floor() as i32;
            let max = max.ceil() as i32;
    
            for i in min..=max {
                let step_size = if i % 3200 == 0 {
                    // 100 tile
                    3200.0
                } else if i % 320 == 0 {
                    // 10 tile
                    320.0
                } else if i % 32 == 0 {
                    // 1 tile
                    32.0
                } else {
                    // skip grids below 1 tile
                    continue;
                };
    
                marks.push(GridMark {
                    value: i as f64,
                    step_size,
                });
            }
    
            marks
        };

        if let Some(texture) = self.texture.as_ref() {
            if self.preview_name == texture.name() {
                let size = texture.size_vec2();

                let image = PlotImage::new(
                    texture,
                    PlotPoint::new(size[0] / 2.0, size[1] / -2.0),
                    size
                );
            
                let plot = Plot::new("image_preview")
                    .auto_bounds([true, true].into())
                    .data_aspect(1.0)
                    .show_background(false)
                    .allow_boxed_zoom(false)
                    .clamp_grid(true)
                    .min_size(egui::vec2(100.0, 100.0))
                    .set_margin_fraction(egui::vec2(0.005, 0.005))
                    .show_axes([false, false])
                    .x_grid_spacer(grid_fmt)
                    .y_grid_spacer(grid_fmt)
                    .label_formatter(label_fmt);

                plot.show(ui, |plot_ui| {
                    plot_ui.image(image.name("Image"));
                    if let Some(rect) = self.selected_region {
                        let [x1, y1] = [rect[0][0] as f64, rect[0][1] as f64];
                        let [x2, y2] = [rect[1][0] as f64 + x1, rect[1][1] as f64 + y1];
                        let points = vec![
                            [x1 * 32.0, y1 * -32.0],
                            [x2 * 32.0 + 32.0, y1 * -32.0],
                            [x2 * 32.0 + 32.0, y2 * -32.0 - 32.0],
                            [x1 * 32.0, y2 * -32.0 - 32.0],
                        ];

                        let rectangle = Polygon::new(points)
                            .stroke(Stroke::new(2.0, egui::Color32::LIGHT_BLUE))
                            .fill_color(egui::Color32::TRANSPARENT);
                        plot_ui.polygon(rectangle);
                    }
                    self.cursor_coords.take();
                    if plot_ui.response().secondary_clicked() {
                        if let Some(pointer) = plot_ui.pointer_coordinate() {
                            self.cursor_coords = Some([(pointer.x/32.0).floor() as u32, (pointer.y/-32.0).floor() as u32]);
                        }
                    }
                    match &self.preview_bounds {
                        Some(bounds) => {
                            plot_ui.set_plot_bounds(*bounds);
                            self.preview_bounds = None;
                        }
                        _ => {}
                    }
                });
            } else {
                //unload texture if it doesn't match what should be loaded.
                self.action = Action::Zoom(PreviewZoom::All);
                self.texture = None;
            }
        } else {
            //load texture based on name if not loaded
            self.action = Action::Zoom(PreviewZoom::All);
            let entry_option = self.loaded_graphics.shared.tile_page_info
                .get_mut(&self.preview_name);

            if let Some(entry) = entry_option {
                let image_path = entry.image_path.clone();

                if entry.texture.is_some() {
                    self.texture = entry.texture.clone();
                } else if image_path.exists() {
                    let dyn_image = image::open(image_path)?;
                    let size = [dyn_image.width() as _, dyn_image.height() as _];
                    let image = dyn_image.as_bytes();
                    let rgba = egui::ColorImage::from_rgba_unmultiplied(size, image);
                    let options = TextureOptions::NEAREST;
                    let new_texture = ui.ctx().load_texture(self.preview_name.clone(), rgba, options);
    
                    entry.texture = Some(new_texture);
                    self.texture = entry.texture.clone();
                } else {
                    self.exception = DFGHError::ImageLoadError(entry.image_path.clone());
                }
            }
        }

        Ok(())
    }
}

impl eframe::App for DFGraphicsHelper {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //Error Window
        if !matches!(self.exception, DFGHError::None) {
            error_window(self, ctx);
        }

        //Draw File menu tab and internal items
        egui::TopBottomPanel::top("top control panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.main_window = MainWindow::DefaultMenu;
                        self.loaded_graphics = Graphics::new();
                        ui.close_menu();
                    }
                    if ui.button("Import From..").clicked() {
                        self.path = PathBuf::new();
                        self.action = Action::Import;
                        ui.close_menu();
                    }
                    if ui.button("Import").clicked() {
                        self.action = Action::Import;
                        ui.close_menu();
                    }
                    if ui.button("Export").clicked() {
                        self.action = Action::Export;
                        ui.close_menu();
                    }
                });
                if ui.button("Update").clicked() {
                    self.action = Action::Update;
                }
            });
        });

        //Draw Main tree Panel
        {egui::SidePanel::left("main_tree")
            .default_width(300.0)
            .resizable(true)
            .show(ctx, |ui| {
            //Draw tree-style selection menu on left side
            egui::ScrollArea::both().show(ui, |ui| {
                self.main_tree(ui, ctx)
            });
        });}

        //Draw Preview Panel
        if self.preview {
            egui::SidePanel::right("preview panel")
                .default_width(600.0)
                .width_range(80.0..=1000.0)
                .resizable(true)
                .show(ctx, |ui| {
                match self.preview_image(ui) {
                    Ok(_) => {},
                    Err(e) => {self.exception = DFGHError::from(e)}
                }
            });
        }

        //Draw Main Panel
        egui::CentralPanel::default().show(ctx,|ui| {
            //Draw main window by matching self.main_window
            egui::ScrollArea::horizontal()
                .show(ui, |ui| {
                let result;
                match self.main_window {
                    MainWindow::TilePageFileDefaultMenu =>  result = self.tile_page_file_default_menu(ui),
                    MainWindow::CreatureDefaultMenu =>      result = self.graphics_default_menu(ui),
                    MainWindow::TilePageFileMenu =>         result = self.tile_page_file_menu(ui),
                    MainWindow::TilePageMenu =>             result = self.tile_page_menu(ui),
                    MainWindow::GraphicsFileMenu =>         result = self.graphics_file_menu(ui),
                    MainWindow::CreatureMenu =>             result = self.graphics_menu(ui),
                    MainWindow::LayerSetMenu =>             result = self.layer_set_menu(ui),
                    MainWindow::LayerGroupMenu =>           result = self.layer_group_menu(ui),
                    MainWindow::LayerMenu =>                result = self.layer_menu(ui),
                    MainWindow::ConditionMenu =>            result = self.condition_menu(ui),
                    MainWindow::ReferenceMenu =>            result = self.default_menu(ui),
                    MainWindow::DefaultMenu =>              result = self.default_menu(ui),
                }
                if result.is_err() {
                    self.exception = result.unwrap_err();
                }
            });
        });

        //Hotkey Handler
        if !ctx.wants_keyboard_input() {
            let undo = &KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::Z,
            };
            if ctx.input_mut(|i| i.consume_shortcut(undo)) {
                self.action = Action::Undo;
            }
            let redo = &KeyboardShortcut {
                modifiers: Modifiers::SHIFT.plus(Modifiers::COMMAND),
                logical_key: Key::Z
            };
            if ctx.input_mut(|i| i.consume_shortcut(redo)) {
                self.action = Action::Redo;
            }
            let import = &KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::I
            };
            if ctx.input_mut(|i| i.consume_shortcut(import)) {
                self.action = Action::Import;
            }
            let open = &KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::O
            };
            if ctx.input_mut(|i| i.consume_shortcut(open)) {
                self.action = Action::Import;
            }
            let export = &KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::E
            };
            if ctx.input_mut(|i| i.consume_shortcut(export)) {
                self.action = Action::Export;
            }
        }

        //Action handler
        {
            let mut result = Ok(());
            match &self.action { //respond to the context menus
                Action::Delete(selected) => {
                    result = self.delete(selected.clone());
                },
                Action::Copy(selected) => {
                    self.copy(selected.clone());
                },
                Action::Cut(selected) => {
                    result = self.cut(selected.clone());
                },
                Action::Paste => {
                    result = self.paste();
                },
                Action::Duplicate(selected) => {
                    self.copy(selected.clone());
                    result = self.paste();
                },
                Action::Insert(kind) => {
                    result = self.insert(kind.clone());
                },
                Action::Undo => {
                    self.undo();
                },
                Action::Redo => {
                    self.redo();
                },
                Action::Import => {
                    self.import();
                },
                Action::Export => {
                    self.export();
                },
                Action::Update => {
                    self.update();
                },
                Action::Zoom(zoom) => {
                    self.zoom(zoom.clone());
                }
                Action::None => {},
            }
            self.action = Action::None;
            if result.is_err() {
                self.exception = result.unwrap_err();
            }
        }
    }
}
