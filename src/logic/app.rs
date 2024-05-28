use egui::{Context, Key, KeyboardShortcut, Modifiers, Sense, Stroke, TextureHandle, TextureOptions, Ui};
use egui_plot::{GridInput, GridMark, Plot, PlotImage, PlotPoint, Polygon};
// use convert_case::{Case, Casing};
use rfd;
use std::path::PathBuf;
use std::path;

use crate::{Plant, Statue, TileGraphic, PADDING};
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
    GraphicsFileDefaultMenu,
    GraphicsFileMenu,
    CreatureMenu,
    LayerGroupMenu,
    LayerSetMenu,
    LayerMenu,
    SimpleLayerMenu,
    StatueMenu,
    PlantMenu,
    TileGraphicMenu,
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
    Statue(Statue),
    Plant(Plant),
    TileGraphic(TileGraphic),
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
            MainWindow::GraphicsFileDefaultMenu => ContextData::Creature(Creature::new()),//todo
            MainWindow::GraphicsFileMenu => ContextData::GraphicsFile(GraphicsFile::default()),
            MainWindow::CreatureMenu => ContextData::Creature(Creature::new()),
            MainWindow::LayerGroupMenu => ContextData::LayerGroup(LayerGroup::new()),
            MainWindow::LayerSetMenu => ContextData::LayerSet(LayerSet::new()),
            MainWindow::LayerMenu => ContextData::Layer(Layer::new()),
            MainWindow::SimpleLayerMenu => ContextData::SimpleLayer(SimpleLayer::new()),
            MainWindow::StatueMenu => ContextData::Statue(Statue::new()),
            MainWindow::PlantMenu => ContextData::Plant(Plant::new()),
            MainWindow::TileGraphicMenu => ContextData::TileGraphic(TileGraphic::new()),
        }
    }
}
impl From<Statue> for ContextData {
    fn from(value: Statue) -> Self {
        let _x = value;
        todo!()
    }
}
impl From<Plant> for ContextData {
    fn from(value: Plant) -> Self {
        let _x = value;
        todo!()
    }
}
impl From<TileGraphic> for ContextData {
    fn from(value: TileGraphic) -> Self {
        let _x = value;
        todo!()
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
    Debug,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GraphicsIndices {
    tile_page_file_index: usize,
    tile_page_index: usize,
    graphics_file_index: usize,
    graphics_index: usize,
    layer_set_index: usize,
    layer_group_index: usize,
    layer_index: usize,
    simple_layer_index: usize,
}
impl GraphicsIndices {
    fn new() -> GraphicsIndices {
        Self {
            tile_page_file_index: 0,
            tile_page_index: 0,
            graphics_file_index: 0,
            graphics_index: 0,
            layer_set_index: 0,
            layer_group_index: 0,
            layer_index: 0,
            simple_layer_index: 0,
        }
    }
}
impl From<[usize; 8]> for GraphicsIndices {
    fn from(index_array: [usize; 8]) -> Self {
        GraphicsIndices {
            tile_page_file_index:   index_array[0],
            tile_page_index:        index_array[1],
            graphics_file_index:    index_array[2],
            graphics_index:         index_array[3],
            layer_set_index:        index_array[4],
            layer_group_index:      index_array[5],
            layer_index:            index_array[6],
            simple_layer_index:     index_array[7],
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
    undo_buffer: Vec<(Vec<TilePageFile>, Vec<GraphicsFile>, GraphicsIndices)>,
    redo_buffer: Vec<(Vec<TilePageFile>, Vec<GraphicsFile>, GraphicsIndices)>,
    pub errors: Vec<DFGHError>,
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
            errors: Vec::new(),
        }
    }

    fn debug(&mut self) {
        dbg!(self.copied.clone());
    }

    fn import(&mut self) {
        self.save_state();
        if !self.path.exists() {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Choose Mod Folder")
                .pick_folder() {
                self.path = path;
            }
        }

        let (graphics, path, mut errors) = Graphics::import(&mut self.path);
        (self.loaded_graphics, self.path) = (graphics, path);
        self.errors.append(&mut errors);

        self.action = Action::None;
    }

    fn export(&mut self) {
        if !self.path.exists() {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Choose Mod Folder")
                .pick_folder() {
                let export_result = self.loaded_graphics.export(&path);
                if export_result.is_err() {
                    self.errors.push(export_result.unwrap_err());
                };
            }
        } else {
            let export_result = self.loaded_graphics.export(&self.path);
            if export_result.is_err() {
                self.errors.push(export_result.unwrap_err());
            };
        }
        self.action = Action::None;
    }

    fn undo(&mut self) {
        if let Some(undo_state) = self.undo_buffer.pop() {
            self.redo_buffer.push(undo_state.clone());

            (
                self.loaded_graphics.tile_page_files,
                self.loaded_graphics.graphics_files,
                self.indices
            ) = undo_state;
        }

        self.action = Action::None;
    }

    fn redo(&mut self) {
        if let Some(redo_state) = self.redo_buffer.pop() {
            self.undo_buffer.push(redo_state.clone());

            (
                self.loaded_graphics.tile_page_files,
                self.loaded_graphics.graphics_files,
                self.indices
            ) = redo_state;
        }

        self.action = Action::None;
    }

    fn context(ui: &mut Ui, selected: ContextData) -> Action {//todo
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
                    ContextData::Condition(_) |
                    ContextData::Statue(_) |
                    ContextData::Plant(_) |
                    ContextData::TileGraphic(_) => {
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
        self.undo_buffer.push((self.loaded_graphics.tile_page_files.clone(), self.loaded_graphics.graphics_files.clone(), self.indices));

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
                } else {
                    tps.push(tile_page_file.clone());
                }
            },
            ContextData::TilePage(tile_page) => {
                let ts = &mut graphics.tile_page_files
                    .get_mut(indices.tile_page_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .tile_pages;
                if indices.tile_page_index < ts.len() {
                    ts.insert(indices.tile_page_index, tile_page.clone());//checked
                } else {
                    ts.push(tile_page.clone());
                }
            },
            ContextData::GraphicsFile(graphics_file) => {
                let gfs = &mut graphics.graphics_files;
                if indices.graphics_file_index < gfs.len() {
                    gfs.insert(indices.graphics_file_index, graphics_file.clone());//checked
                } else {
                    gfs.push(graphics_file.clone());
                }
            },
            ContextData::Creature(creature) => {
                if let GraphicsFile::CreatureFile(_, cs) = &mut graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    if indices.graphics_index < cs.len() {
                        cs.insert(indices.graphics_index, creature.clone());//checked
                    } else {
                        cs.push(creature.clone());
                    }
                }
            },
            ContextData::LayerSet(layer_set) => {
                if let GraphicsFile::CreatureFile(_, cs) = &mut graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    let lss = &mut cs.get_mut(indices.graphics_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_sets;
                    if indices.layer_set_index < lss.len() {
                        lss.insert(indices.layer_set_index, layer_set.clone());//checked
                    } else {
                        lss.push(layer_set.clone());
                    }
                }
            },
            ContextData::LayerGroup(layer_group) => {
                if let GraphicsFile::CreatureFile(_, cs) = &mut graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    let lgs = &mut cs.get_mut(indices.graphics_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_sets
                        .get_mut(indices.layer_set_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_groups;
                    if indices.layer_group_index < lgs.len() {
                        lgs.insert(indices.layer_group_index, layer_group.clone());//checked
                    } else {
                        lgs.push(layer_group.clone());
                    }
                }
            },
            ContextData::Layer(layer) => {
                if let GraphicsFile::CreatureFile(_, cs) = &mut graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    let ls = &mut cs.get_mut(indices.graphics_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_sets
                        .get_mut(indices.layer_set_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_groups
                        .get_mut(indices.layer_group_index)
                        .ok_or(DFGHError::IndexError)?
                        .layers;
                    if indices.layer_index < ls.len() {
                        ls.insert(indices.layer_index, layer.clone());//checked
                    } else {
                        ls.push(layer.clone());
                    }
                }
            },
            ContextData::Condition(condition) => {
                if let GraphicsFile::CreatureFile(_, cs) = &mut graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    let conds = &mut cs.get_mut(indices.graphics_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_sets
                        .get_mut(indices.layer_set_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_groups
                        .get_mut(indices.layer_group_index)
                        .ok_or(DFGHError::IndexError)?
                        .layers
                        .get_mut(indices.layer_index)
                        .ok_or(DFGHError::IndexError)?
                        .conditions;
                    conds.push(condition.clone());
                }
            },
            ContextData::SimpleLayer(simple_layer) => {
                if let GraphicsFile::CreatureFile(_, cs) = &mut graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    let sls = &mut cs.get_mut(indices.graphics_index)
                        .ok_or(DFGHError::IndexError)?
                        .simple_layers;
                    if indices.simple_layer_index < sls.len() {
                        sls.insert(indices.simple_layer_index, simple_layer.clone());//checked
                    } else {
                        sls.push(simple_layer.clone());
                    }
                }
            },
            ContextData::Statue(statue) => {
                if let GraphicsFile::StatueCreatureFile(_, ss) = &mut graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    if indices.graphics_index < ss.len() {
                        ss.insert(indices.graphics_index, statue.clone());//checked
                    } else {
                        ss.push(statue.clone());
                    }
                }
            },
            ContextData::Plant(plant) => {
                if let GraphicsFile::PlantFile(_, ps) = &mut graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    if indices.graphics_index < ps.len() {
                        ps.insert(indices.graphics_index, plant.clone());//checked
                    } else {
                        ps.push(plant.clone());
                    }
                }
            },
            ContextData::TileGraphic(tile_graphic) => {
                if let GraphicsFile::TileGraphicsFile(_, tgs) = &mut graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    if indices.graphics_index < tgs.len() {
                        tgs.insert(indices.graphics_index, tile_graphic.clone());//checked
                    } else {
                        tgs.push(tile_graphic.clone());
                    }
                }
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
                        self.main_window = MainWindow::GraphicsFileDefaultMenu;
                    }
                }
            },
            ContextData::Creature(_) => {
                if let GraphicsFile::CreatureFile(_, cs) = graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    if indices.graphics_index < cs.len() {
                        cs.remove(indices.graphics_index);//checked
                        if indices.graphics_index >= 1 {
                            indices.graphics_index -= 1;
                        } else {
                            self.main_window = MainWindow::GraphicsFileMenu;
                        }
                    }
                }
            },
            ContextData::LayerSet(_) => {
                if let GraphicsFile::CreatureFile(_, cs) = graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    let lss = &mut cs
                        .get_mut(indices.graphics_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_sets;
                    if indices.layer_set_index < lss.len() {
                        lss.remove(indices.layer_set_index);//checked
                        if indices.layer_set_index >= 1 {
                            indices.layer_set_index -= 1;
                        } else {
                            self.main_window = MainWindow::CreatureMenu;
                        }
                    }
                }
            },
            ContextData::LayerGroup(_) => {
                if let GraphicsFile::CreatureFile(_, cs) = graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    let lgs = &mut cs
                        .get_mut(indices.graphics_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_sets
                        .get_mut(indices.layer_set_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_groups;
                    if indices.layer_group_index < lgs.len() {
                        lgs.remove(indices.layer_group_index);//checked
                        if indices.layer_group_index >= 1 {
                            indices.layer_group_index -= 1;
                        } else {
                            self.main_window = MainWindow::LayerSetMenu;
                        }
                    }
                }
            },
            ContextData::Layer(_) => {
                if let GraphicsFile::CreatureFile(_, cs) = graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    let ls = &mut cs
                        .get_mut(indices.graphics_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_sets
                        .get_mut(indices.layer_set_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_groups
                        .get_mut(indices.layer_group_index)
                        .ok_or(DFGHError::IndexError)?
                        .layers;
                    if indices.layer_index < ls.len() {
                        ls.remove(indices.layer_index);//checked
                        if indices.layer_index >= 1 {
                            indices.layer_index -= 1;
                        } else {
                            self.main_window = MainWindow::LayerGroupMenu;
                        }
                    }
                }
            },
            ContextData::Condition(_) => {
                if let GraphicsFile::CreatureFile(_, cs) = graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    let conds = &mut cs
                        .get_mut(indices.graphics_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_sets
                        .get_mut(indices.layer_set_index)
                        .ok_or(DFGHError::IndexError)?
                        .layer_groups
                        .get_mut(indices.layer_group_index)
                        .ok_or(DFGHError::IndexError)?
                        .layers.get_mut(indices.layer_index)
                        .ok_or(DFGHError::IndexError)?
                        .conditions;
                    conds.clear();
                }
            },
            ContextData::SimpleLayer(_) => {
                if let GraphicsFile::CreatureFile(_, cs) = graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    let sls = &mut cs
                        .get_mut(indices.graphics_index)
                        .ok_or(DFGHError::IndexError)?
                        .simple_layers;
                    if indices.simple_layer_index < sls.len() {
                        sls.remove(indices.simple_layer_index);//checked
                        if indices.simple_layer_index >= 1 {
                            indices.simple_layer_index -= 1;
                        } else {
                            self.main_window = MainWindow::CreatureMenu;
                        }
                    }
                }
            },
            ContextData::Statue(_) => {
                if let GraphicsFile::StatueCreatureFile(_, ss) = graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    if indices.graphics_index < ss.len() {
                        ss.remove(indices.graphics_index);//checked
                        if indices.graphics_index >= 1 {
                            indices.graphics_index -= 1;
                        } else {
                            self.main_window = MainWindow::GraphicsFileMenu;
                        }
                    }
                }
            },
            ContextData::Plant(_) => {
                if let GraphicsFile::PlantFile(_, ps) = graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    if indices.graphics_index < ps.len() {
                        ps.remove(indices.graphics_index);//checked
                        if indices.graphics_index >= 1 {
                            indices.graphics_index -= 1;
                        } else {
                            self.main_window = MainWindow::GraphicsFileMenu;
                        }
                    }
                }
            },
            ContextData::TileGraphic(_) => {
                if let GraphicsFile::TileGraphicsFile(_, tgs) = graphics.graphics_files
                    .get_mut(indices.graphics_file_index)
                    .ok_or(DFGHError::IndexError)? {
                    if indices.graphics_index < tgs.len() {
                        tgs.remove(indices.graphics_index);//checked
                        if indices.graphics_index >= 1 {
                            indices.graphics_index -= 1;
                        } else {
                            self.main_window = MainWindow::GraphicsFileMenu;
                        }
                    }
                }
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
        //tile page files
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
                    format!("{}", tile_page_file.name.clone().replace("tile_page_", "")))
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
                //tile pages
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
        if ui.add(egui::Label::new("Graphics Files")
            .wrap(false)
            .sense(Sense::click()))
            .clicked()
        {
            self.main_window = MainWindow::GraphicsFileDefaultMenu;
        };
        // // graphics files
        for (i_file, graphics_file) in self.loaded_graphics.graphics_files.iter_mut().enumerate() {
            let id_gf = ui.make_persistent_id(
                format!("graphics_file{}",
                i_file)
            );
            egui::collapsing_header::CollapsingState::load_with_default_open(
                ctx,
                id_gf,
                false,
            )
            .show_header(ui, |ui| {
                let graphics_file_response = ui.add(egui::Label::new(
                    format!("{}", &graphics_file.name()))
                    .sense(Sense::click()));
                if graphics_file_response.clicked() {
                    self.indices = [0, 0, i_file, 0, 0, 0, 0, 0].into();
                    self.main_window = MainWindow::GraphicsFileMenu;
                }
                graphics_file_response.context_menu(|ui| {
                    self.indices = [0, 0, i_file, 0, 0, 0, 0, 0].into();
                    self.action = Self::context(ui, ContextData::from(graphics_file.clone()));
                });
            })
            .body(|ui| {
                match graphics_file {
                    GraphicsFile::DefaultFile => {/*do nothing*/},
                    GraphicsFile::CreatureFile(_, creatures) => {
                        //creatures
                        for (i_creature, creature) in creatures.iter_mut().enumerate() {
                            let id_c = ui.make_persistent_id(
                                format!("graphics{}{}",
                                i_file, i_creature)
                            );
                            egui::collapsing_header::CollapsingState::load_with_default_open(
                                ctx,
                                id_c,
                                false)
                                .show_header(ui, |ui| {
                                let creature_response = ui.add(egui::Label::new(
                                    format!("{}", creature.name))
                                    .sense(Sense::click()));
                                if creature_response.clicked() {
                                    self.indices = [0, 0, i_file, i_creature, 0, 0, 0, 0].into();
                                    self.main_window = MainWindow::CreatureMenu;
                                }
                                creature_response.context_menu(|ui| {
                                    self.indices = [0, 0, i_file, i_creature, 0, 0, 0, 0].into();
                                    self.action = Self::context(ui, ContextData::from(creature.clone()));
                                });
                            }).body(|ui| {
                                //simple layers
                                for (i_simple_layer, simple_layer) in creature.simple_layers.iter_mut().enumerate() {
                                    let simple_layer_response = ui.add(egui::Label::new(
                                        format!("{} {}",
                                        simple_layer.state.name(),
                                        simple_layer.sub_state.clone().map_or("".to_string(), |ss| ss.name())))
                                        .sense(Sense::click())
                                    );
                                    if simple_layer_response.clicked() {
                                        self.indices = [0, 0, i_file, i_creature, 0, 0, 0, i_simple_layer].into();
                                        self.main_window = MainWindow::SimpleLayerMenu;
                                    }
                                    simple_layer_response.context_menu(|ui| {
                                        self.indices = [0, 0, i_file, i_creature, 0, 0, 0, i_simple_layer].into();
                                        self.action = Self::context(ui, ContextData::from(simple_layer.clone()));
                                    });
                                }
                                //layer sets
                                for (i_layer_set, layer_set) in creature.layer_sets.iter_mut().enumerate() {
                                    let id_ls = ui.make_persistent_id(
                                        format!("layer_set{}{}{}",
                                        i_file, i_creature, i_layer_set)
                                    );
                                    egui::collapsing_header::CollapsingState::load_with_default_open(
                                        ctx,
                                        id_ls,
                                        false)
                                        .show_header(ui, |ui| {
                                        let layer_set_response = ui.add(egui::Label::new(
                                            format!("{}", layer_set.state.name()))
                                            .sense(Sense::click()));
                                        if layer_set_response.clicked() {
                                            self.indices = [0, 0, i_file, i_creature, i_layer_set, 0, 0, 0].into();
                                            self.main_window = MainWindow::LayerSetMenu;
                                        }
                                        layer_set_response.context_menu(|ui| {
                                            self.indices = [0, 0, i_file, i_creature, i_layer_set, 0, 0, 0].into();
                                            self.action = Self::context(ui, ContextData::from(layer_set.clone()));
                                        });
                                    }).body(|ui| {
                                        //layer groups
                                        for (i_layer_group, layer_group) in layer_set.layer_groups.iter_mut().enumerate() {
                                            let id_lg = ui.make_persistent_id(
                                                format!("layer_group{}{}{}{}",
                                                i_file, i_creature, i_layer_set, i_layer_group)
                                            );
                                            egui::collapsing_header::CollapsingState::load_with_default_open(
                                                ctx,
                                                id_lg,
                                                false)
                                                .show_header(ui, |ui|
                                                {
                                                let layer_group_response = ui.add(egui::Label::new(
                                                    format!("{}", layer_group.name))
                                                    .sense(Sense::click()));
                                                if layer_group_response.clicked() {
                                                    self.indices = [0, 0, i_file, i_creature, i_layer_set, i_layer_group, 0, 0].into();
                                                    self.main_window = MainWindow::LayerGroupMenu;
                                                }
                                                layer_group_response.context_menu(|ui| {
                                                    self.indices = [0, 0, i_file, i_creature, i_layer_set, i_layer_group, 0, 0].into();
                                                    self.action = Self::context(ui, ContextData::from(layer_group.clone()));
                                                });
                                            }).body(|ui| {
                                                //layers
                                                for (i_layer, layer) in layer_group.layers.iter_mut().enumerate() {
                                                    let id_l = ui.make_persistent_id(
                                                        format!("layer{}{}{}{}{}",
                                                        i_file, i_creature, i_layer_set, i_layer_group, i_layer)
                                                    );
                                                    egui::collapsing_header::CollapsingState::load_with_default_open(ctx,
                                                        id_l,
                                                        false)
                                                        .show_header(ui, |ui|
                                                        {
                                                            let layer_response = ui.add(egui::Label::new(
                                                                format!("{}", &layer.name))
                                                                .sense(Sense::click()));
                                                            if layer_response.clicked() {
                                                                self.indices = [0, 0, i_file, i_creature, i_layer_set, i_layer_group, i_layer, 0].into();
                                                                self.main_window = MainWindow::LayerMenu;
                                                            }
                                                            layer_response.context_menu(|ui| {
                                                                self.indices = [0, 0, i_file, i_creature, i_layer_set, i_layer_group, i_layer, 0].into();
                                                                self.action = Self::context(ui, ContextData::from(layer.clone()));
                                                            });
                                                        })
                                                        .body(|ui|
                                                        {
                                                        //conditions (no separate menu)
                                                        for condition in layer.conditions.iter_mut() {
                                                            let condition_response = ui.add(egui::Label::new(
                                                                format!("\t{}", condition.name()))
                                                                .wrap(false)
                                                                .sense(Sense::click()));
                                                            if condition_response.clicked() {
                                                                self.indices = [0, 0, i_file, i_creature, i_layer_set, i_layer_group, i_layer, 0].into();
                                                                self.main_window = MainWindow::LayerMenu;
                                                            }
                                                            condition_response.context_menu(|ui| {
                                                                self.indices = [0, 0, i_file, i_creature, i_layer_set, i_layer_group, i_layer, 0].into();
                                                                self.action = Self::context(ui, ContextData::from(condition.clone()));
                                                            });
                                                        }
                                                    });
                                                }
                                            });
                                        }
                                    });
                                }
                            });
                        }
                    },
                    GraphicsFile::StatueCreatureFile(_, statues) => {
                        for (i_statue, statue) in statues.iter_mut().enumerate() {
                            let statue_response = ui.add(egui::Label::new(
                                format!("{} {}",
                                statue.name,
                                statue.caste.clone().map_or("".to_string(), |c| c.name())))
                                .sense(Sense::click())
                            );
                            if statue_response.clicked() {
                                self.indices = [0, 0, i_file, i_statue, 0, 0, 0, 0].into();
                                self.main_window = MainWindow::StatueMenu;
                            }
                            statue_response.context_menu(|ui| {
                                self.indices = [0, 0, i_file, i_statue, 0, 0, 0, 0].into();
                                self.action = Self::context(ui, ContextData::from(statue.clone()));
                            });
                        }
                    },
                    GraphicsFile::PlantFile(_, plants) => {
                        for (i_plant, plant) in plants.iter_mut().enumerate() {
                            let plant_response = ui.add(egui::Label::new(
                                format!("{}", plant.name))
                                .sense(Sense::click())
                            );
                            if plant_response.clicked() {
                                self.indices = [0, 0, i_file, i_plant, 0, 0, 0, 0].into();
                                self.main_window = MainWindow::PlantMenu;
                            }
                            plant_response.context_menu(|ui| {
                                self.indices = [0, 0, i_file, i_plant, 0, 0, 0, 0].into();
                                self.action = Self::context(ui, ContextData::from(plant.clone()));
                            });
                        }
                    },
                    GraphicsFile::TileGraphicsFile(_, tile_graphics) => {
                        for (i_tile_graphic, tile_graphics) in tile_graphics.iter_mut().enumerate() {
                            let tile_graphics_response = ui.add(egui::Label::new(
                                format!("{}", tile_graphics.name))
                                .sense(Sense::click())
                            );
                            if tile_graphics_response.clicked() {
                                self.indices = [0, 0, i_file, i_tile_graphic, 0, 0, 0, 0].into();
                                self.main_window = MainWindow::TileGraphicMenu;
                            }
                            tile_graphics_response.context_menu(|ui| {
                                self.indices = [0, 0, i_file, i_tile_graphic, 0, 0, 0, 0].into();
                                self.action = Self::context(ui, ContextData::from(tile_graphics.clone()));
                            });
                        }
                    },
                }
            });
        }
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
        self.selected_region = None;

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
        self.selected_region = None;

        Ok(())
    }

    fn tile_page_file_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Tile Page File Menu\t\t");

            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::TilePageFile(TilePageFile::new()));
            }
        });

        let indices = &mut self.indices;

        if self.loaded_graphics.tile_page_files.is_empty() {
            self.action = Action::Insert(ContextData::TilePageFile(TilePageFile::new()));
        } else {
            let tile_page_file = self
                .loaded_graphics
                .tile_page_files
                .get_mut(indices.tile_page_file_index)
                .ok_or(DFGHError::IndexError)?;

            ui.separator();
            ui.text_edit_singleline(&mut tile_page_file.name);
            ui.add_space(PADDING);

            if ui.button("New Tile Page").clicked() {
                self.action = Action::Insert(ContextData::TilePage(TilePage::new()));
            }
        }

        self.preview = false;
        self.preview_name = String::new();
        self.selected_region = None;
        
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
            self.selected_region = None;
        }
        Ok(())
    }

    fn graphics_file_default_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.label("Graphics File Menu");
        ui.separator();

        if ui.small_button("New Graphics File").clicked() {
            self.action = Action::Insert(ContextData::GraphicsFile(GraphicsFile::new()));
        }

        self.preview = false;
        self.preview_name = String::new();
        self.selected_region = None;

        Ok(())
    }

    fn graphics_file_menu(&mut self, ui: &mut Ui) -> Result<()> {
        let indices = &mut self.indices;
        let graphics_file = self.loaded_graphics.graphics_files
            .get_mut(indices.graphics_file_index)
            .ok_or(DFGHError::IndexError)?;

        match graphics_file {
            GraphicsFile::DefaultFile => {
                ui.horizontal(|ui| {
                    ui.label("Graphics File Menu");
                    if ui.button("Delete").clicked() {
                        self.action = Action::Delete(ContextData::GraphicsFile(GraphicsFile::new()));
                    }
                });
                ui.separator();
                ui.add_space(PADDING);
                egui::ComboBox::from_label("Graphics Type")
                    .selected_text(&graphics_file.name())
                    .show_ui(ui, |ui| {
                    ui.selectable_value(graphics_file,
                        GraphicsFile::CreatureFile("(new)".to_string(), Vec::new()),
                        "Creature"
                    );
                    ui.selectable_value(graphics_file,
                        GraphicsFile::StatueCreatureFile("(new)".to_string(), Vec::new()),
                        "Statue"
                    );
                    ui.selectable_value(graphics_file,
                        GraphicsFile::PlantFile("(new)".to_string(), Vec::new()),
                        "Plant"
                    );
                    ui.selectable_value(graphics_file,
                        GraphicsFile::TileGraphicsFile("(new)".to_string(), Vec::new()),
                        "Tile Graphic"
                    );
                });
            },
            GraphicsFile::CreatureFile(name, _) => {
                ui.horizontal(|ui| {
                    ui.label("Creature File Menu");
                    if ui.button("Delete").clicked() {
                        self.action = Action::Delete(ContextData::GraphicsFile(GraphicsFile::new()));
                    }
                });

                ui.add_space(PADDING);
                ui.label("File Name:");
                ui.text_edit_singleline(name);

                ui.add_space(PADDING);
                if ui.button("New Creature").clicked() {
                    self.action = Action::Insert(ContextData::Creature(Creature::new()));
                }
            },
            GraphicsFile::StatueCreatureFile(name, _) => {
                ui.horizontal(|ui| {
                    ui.label("Statue File Menu");
                    if ui.button("Delete").clicked() {
                        self.action = Action::Delete(ContextData::GraphicsFile(GraphicsFile::new()));
                    }
                });

                ui.add_space(PADDING);
                ui.label("File Name:");
                ui.text_edit_singleline(name);

                ui.add_space(PADDING);
                if ui.button("New Creature Statue").clicked() {
                    self.action = Action::Insert(ContextData::Statue(Statue::new()));
                }
            },
            GraphicsFile::PlantFile(name, _) => {
                ui.horizontal(|ui| {
                    ui.label("Plant File Menu");
                    if ui.button("Delete").clicked() {
                        self.action = Action::Delete(ContextData::GraphicsFile(GraphicsFile::new()));
                    }
                });

                ui.add_space(PADDING);
                ui.label("File Name:");
                ui.text_edit_singleline(name);

                ui.add_space(PADDING);
                if ui.button("New Plant").clicked() {
                    self.action = Action::Insert(ContextData::Plant(Plant::new()));
                }
            },
            GraphicsFile::TileGraphicsFile(name, _) => {
                ui.horizontal(|ui| {
                    ui.label("Tile Graphic File Menu");
                    if ui.button("Delete").clicked() {
                        self.action = Action::Delete(ContextData::GraphicsFile(GraphicsFile::new()));
                    }
                });

                ui.add_space(PADDING);
                ui.label("File Name:");
                ui.text_edit_singleline(name);

                ui.add_space(PADDING);
                if ui.button("New Tile Graphic").clicked() {
                    self.action = Action::Insert(ContextData::TileGraphic(TileGraphic::new()));
                }
            },
        }

        self.preview = false;
        self.preview_name = String::new();
        self.selected_region = None;
        
        Ok(())
    }

    fn graphics_menu(&mut self, ui: &mut Ui) -> Result<()> {
        let indices = &mut self.indices;
        let shared = &mut self.loaded_graphics.shared;
        let graphics_file = self.loaded_graphics.graphics_files
            .get_mut(indices.graphics_file_index)
            .ok_or(DFGHError::IndexError)?;
        match graphics_file {
            GraphicsFile::DefaultFile => {
                ui.horizontal(|ui| {
                    ui.label("Graphics Menu");
                    if ui.button("Delete").clicked() {
                        self.action = Action::Delete(ContextData::Creature(Creature::new()));
                    }
                });
                ui.separator();
                ui.add_space(PADDING);
                egui::ComboBox::from_label("Graphics Type")
                    .selected_text(&graphics_file.name())
                    .show_ui(ui, |ui| {
                    ui.selectable_value(graphics_file,
                        GraphicsFile::CreatureFile("(new)".to_string(), Vec::new()),
                        "Creature"
                    );
                    ui.selectable_value(graphics_file,
                        GraphicsFile::StatueCreatureFile("(new)".to_string(), Vec::new()),
                        "Statue"
                    );
                    ui.selectable_value(graphics_file,
                        GraphicsFile::PlantFile("(new)".to_string(), Vec::new()),
                        "Plant"
                    );
                    ui.selectable_value(graphics_file,
                        GraphicsFile::TileGraphicsFile("(new)".to_string(), Vec::new()),
                        "Tile Graphic"
                    );
                });

                self.preview = false;
                self.preview_name = String::new();
                self.selected_region = None;
                
                return Ok(())
            },
            GraphicsFile::CreatureFile(_, creatures) => {
                ui.horizontal(|ui| {
                    ui.label("Creature Menu");
                    if ui.button("Delete").clicked() {
                        self.action = Action::Delete(ContextData::Creature(Creature::new()));
                    }
                });

                let creature = creatures
                    .get_mut(indices.graphics_index)
                    .ok_or(DFGHError::IndexError)?;

                creature.menu(ui, shared);

                self.preview = true;
                self.preview_name = "Creature Preview".to_string();
                self.selected_region = None;
                
                return Ok(())
            },
            GraphicsFile::StatueCreatureFile(_, statues) => {
                ui.horizontal(|ui| {
                    ui.label("Statue Menu");
                    if ui.button("Delete").clicked() {
                        self.action = Action::Delete(ContextData::Statue(Statue::new()));
                    }
                });

                let statue = statues
                    .get_mut(indices.graphics_index)
                    .ok_or(DFGHError::IndexError)?;

                statue.menu(ui, shared);

                self.preview = true;
                self.preview_name = "Statue Preview".to_string();
                self.selected_region = None;
                
                return Ok(())
            },
            GraphicsFile::PlantFile(_, plants) => {
                ui.horizontal(|ui| {
                    ui.label("Plant Menu");
                    if ui.button("Delete").clicked() {
                        self.action = Action::Delete(ContextData::Plant(Plant::new()));
                    }
                });

                let plant = plants
                    .get_mut(indices.graphics_index)
                    .ok_or(DFGHError::IndexError)?;

                plant.menu(ui, shared);

                self.preview = true;
                self.preview_name = "Plant Preview".to_string();
                self.selected_region = None;
                
                return Ok(())
            },
            GraphicsFile::TileGraphicsFile(_, tile_graphics) => {
                ui.horizontal(|ui| {
                    ui.label("Tile Graphic Menu");
                    if ui.button("Delete").clicked() {
                        self.action = Action::Delete(ContextData::TileGraphic(TileGraphic::new()));
                    }
                });

                let tile_graphic = tile_graphics
                    .get_mut(indices.graphics_index)
                    .ok_or(DFGHError::IndexError)?;

                tile_graphic.menu(ui, shared);

                self.preview = false;
                self.preview_name = String::new();
                self.selected_region = None;
                
                return Ok(())
            },
        }
    }

    fn layer_set_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Layer Set Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::LayerGroup(LayerGroup::new()));
            }
        });
        
        let indices = &mut self.indices;

        if let GraphicsFile::CreatureFile(_, creatures) = &mut self
            .loaded_graphics
            .graphics_files
            .get_mut(indices.graphics_file_index)
            .ok_or(DFGHError::IndexError)? {
            let layer_sets = &mut creatures
                .get_mut(indices.graphics_index)
                .ok_or(DFGHError::IndexError)?
                .layer_sets;
            if layer_sets.is_empty() {
                if ui.small_button("Create Layer Set").clicked() {
                    self.action = Action::Insert(ContextData::LayerSet(LayerSet::new()));
                }
            } else {
                let layer_set = layer_sets
                    .get_mut(indices.layer_set_index)
                    .ok_or(DFGHError::IndexError)?;
    
                let shared = &mut self.loaded_graphics.shared;
    
                layer_set.menu(ui, shared);
    
                self.preview = false;
                self.preview_name = String::new();
                self.selected_region = None;
            }
        }
        Ok(())
    }

    fn layer_group_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Layer Group Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::LayerGroup(LayerGroup::new()));
            }
        });
        
        let indices = &mut self.indices;

        if let GraphicsFile::CreatureFile(_, creatures) = &mut self
            .loaded_graphics
            .graphics_files
            .get_mut(indices.graphics_file_index)
            .ok_or(DFGHError::IndexError)? {
            let layer_groups = &mut creatures
                .get_mut(indices.graphics_index)
                .ok_or(DFGHError::IndexError)?
                .layer_sets
                .get_mut(indices.layer_set_index)
                .ok_or(DFGHError::IndexError)?
                .layer_groups;
            if layer_groups.is_empty() {
                if ui.small_button("Create Layer Group").clicked() {
                    self.action = Action::Insert(ContextData::LayerSet(LayerSet::new()));
                }
            } else {
                let layer_group = layer_groups
                    .get_mut(indices.layer_group_index)
                    .ok_or(DFGHError::IndexError)?;
    
                let shared = &mut self.loaded_graphics.shared;
    
                layer_group.menu(ui, shared);
    
                self.preview = false;
                self.preview_name = String::new();
                self.selected_region = None;
            }
        }
        Ok(())
    }

    fn layer_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Layer Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::Layer(Layer::new()));
            }
        });
        
        let indices = &mut self.indices;

        if let GraphicsFile::CreatureFile(_, creatures) = &mut self
            .loaded_graphics
            .graphics_files
            .get_mut(indices.graphics_file_index)
            .ok_or(DFGHError::IndexError)? {
            let layers = &mut creatures
                .get_mut(indices.graphics_index)
                .ok_or(DFGHError::IndexError)?
                .layer_sets
                .get_mut(indices.layer_set_index)
                .ok_or(DFGHError::IndexError)?
                .layer_groups
                .get_mut(indices.layer_group_index)
                .ok_or(DFGHError::IndexError)?
                .layers;
            if layers.is_empty() {
                if ui.small_button("Create Layer").clicked() {
                    self.action = Action::Insert(ContextData::Layer(Layer::new()));
                }
            } else {
                let layer = layers
                    .get_mut(indices.layer_index)
                    .ok_or(DFGHError::IndexError)?;
    
                let shared = &mut self.loaded_graphics.shared;
    
                layer.menu(ui, shared);
    
                self.preview = true;
                self.preview_name = layer.tile_name.clone();
                self.selected_region = Some([layer.coords, layer.large_coords.unwrap_or([0,0])]);
                if let Some(coords) = self.cursor_coords {
                    layer.coords = coords;
                }
            }
        }
        Ok(())
    }

    fn simple_layer_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Layer Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::SimpleLayer(SimpleLayer::new()));
            }
        });
        
        let indices = &mut self.indices;

        if let GraphicsFile::CreatureFile(_, creatures) = &mut self
            .loaded_graphics
            .graphics_files
            .get_mut(indices.graphics_file_index)
            .ok_or(DFGHError::IndexError)? {
            let simple_layers = &mut creatures
                .get_mut(indices.graphics_index)
                .ok_or(DFGHError::IndexError)?
                .simple_layers;
            if simple_layers.is_empty() {
                if ui.small_button("Create Layer").clicked() {
                    self.action = Action::Insert(ContextData::SimpleLayer(SimpleLayer::new()));
                }
            } else {
                let simple_layer = simple_layers
                    .get_mut(indices.simple_layer_index)
                    .ok_or(DFGHError::IndexError)?;
    
                let shared = &mut self.loaded_graphics.shared;
    
                simple_layer.menu(ui, shared);
    
                self.preview = true;
                self.preview_name = simple_layer.tile_name.clone();
                self.selected_region = Some([simple_layer.coords, simple_layer.large_coords.unwrap_or([0,0])]);
                if let Some(coords) = self.cursor_coords {
                    simple_layer.coords = coords;
                }
            }
        }
        Ok(())
    }

    fn statue_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Creature Statue Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::Statue(Statue::new()));
            }
        });
        
        let indices = &mut self.indices;

        if let GraphicsFile::StatueCreatureFile(_, statues) = &mut self
            .loaded_graphics
            .graphics_files
            .get_mut(indices.graphics_file_index)
            .ok_or(DFGHError::IndexError)? {
            if statues.is_empty() {
                if ui.small_button("Create Statue").clicked() {
                    self.action = Action::Insert(ContextData::Statue(Statue::new()));
                }
            } else {
                let statue = statues
                    .get_mut(indices.graphics_index)
                    .ok_or(DFGHError::IndexError)?;
    
                let shared = &mut self.loaded_graphics.shared;
    
                statue.menu(ui, shared);
    
                self.preview = true;
                self.preview_name = statue.tile_name.clone();
                self.selected_region = Some([statue.coords, statue.large_coords.unwrap_or([0,0])]);
                if let Some(coords) = self.cursor_coords {
                    statue.coords = coords;
                }
            }
        }
        Ok(())
    }

    fn plant_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Plant Graphics Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::Plant(Plant::new()));
            }
        });
        
        let indices = &mut self.indices;

        if let GraphicsFile::PlantFile(_, plants) = &mut self
            .loaded_graphics
            .graphics_files
            .get_mut(indices.graphics_file_index)
            .ok_or(DFGHError::IndexError)? {
            if plants.is_empty() {
                if ui.small_button("Create Plant").clicked() {
                    self.action = Action::Insert(ContextData::Plant(Plant::new()));
                }
            } else {
                let plant = plants
                    .get_mut(indices.graphics_index)
                    .ok_or(DFGHError::IndexError)?;
    
                let shared = &mut self.loaded_graphics.shared;
    
                plant.menu(ui, shared);
    
                self.preview = true;
                self.preview_name = plant.tile_name.clone();
                self.selected_region = None;
                if let Some(coords) = self.cursor_coords {
                    plant.coords[0] = Some(coords);
                }
            }
        }
        Ok(())
    }

    fn tile_graphic_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Tile Graphic Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::TileGraphic(TileGraphic::new()));
            }
        });
        
        let indices = &mut self.indices;

        if let GraphicsFile::TileGraphicsFile(_, tile_graphics) = &mut self
            .loaded_graphics
            .graphics_files
            .get_mut(indices.graphics_file_index)
            .ok_or(DFGHError::IndexError)? {
            if tile_graphics.is_empty() {
                if ui.small_button("Create Tile Graphic").clicked() {
                    self.action = Action::Insert(ContextData::TileGraphic(TileGraphic::new()));
                }
            } else {
                let tile_graphic = tile_graphics
                    .get_mut(indices.graphics_index)
                    .ok_or(DFGHError::IndexError)?;
    
                let shared = &mut self.loaded_graphics.shared;
    
                tile_graphic.menu(ui, shared);
    
                self.preview = true;
                self.preview_name = tile_graphic.tile_name.clone();
                self.selected_region = Some([tile_graphic.coords, [0, 0]]);
                if let Some(coords) = self.cursor_coords {
                    tile_graphic.coords = coords;
                }
            }
        }
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
                Err(e) => {self.errors.push(DFGHError::from(e))}
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
                    self.errors.push(DFGHError::ImageLoadError(entry.image_path.clone()));
                }
            }
        }

        Ok(())
    }
}

impl eframe::App for DFGraphicsHelper {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //Error Window
        if self.errors.last().is_some() {
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
                    self.loaded_graphics.shared.clear();
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
                    Err(e) => {self.errors.push(DFGHError::from(e))}
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
                    MainWindow::GraphicsFileDefaultMenu =>  result = self.graphics_file_default_menu(ui),
                    MainWindow::TilePageFileMenu =>         result = self.tile_page_file_menu(ui),
                    MainWindow::TilePageMenu =>             result = self.tile_page_menu(ui),
                    MainWindow::GraphicsFileMenu =>         result = self.graphics_file_menu(ui),
                    MainWindow::CreatureMenu =>             result = self.graphics_menu(ui),
                    MainWindow::LayerSetMenu =>             result = self.layer_set_menu(ui),
                    MainWindow::LayerGroupMenu =>           result = self.layer_group_menu(ui),
                    MainWindow::LayerMenu =>                result = self.layer_menu(ui),
                    MainWindow::SimpleLayerMenu =>          result = self.simple_layer_menu(ui),//todo
                    MainWindow::StatueMenu =>               result = self.statue_menu(ui),
                    MainWindow::PlantMenu =>                result = self.plant_menu(ui),//todo
                    MainWindow::TileGraphicMenu =>          result = self.tile_graphic_menu(ui),//todo
                    MainWindow::DefaultMenu =>              result = self.default_menu(ui),
                }
                if result.is_err() {
                    self.errors.push(result.unwrap_err());
                }
            });
        });

        //Hotkey Handler
        if !ctx.wants_keyboard_input() {
            let redo = &KeyboardShortcut {
                modifiers: Modifiers::SHIFT.plus(Modifiers::COMMAND),
                logical_key: Key::Z
            };
            if ctx.input_mut(|i| i.consume_shortcut(redo)) {
                self.action = Action::Redo;
            }
            let undo = &KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::Z,
            };
            if ctx.input_mut(|i| i.consume_shortcut(undo)) {
                self.action = Action::Undo;
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
            let debug = &KeyboardShortcut {
                modifiers: Modifiers::COMMAND,
                logical_key: Key::P
            };
            if ctx.input_mut(|i| i.consume_shortcut(debug)) {
                self.action = Action::Debug;
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
                },
                Action::Debug => {
                    self.debug();
                }
                Action::None => {},
            }
            self.action = Action::None;
            if result.is_err() {
                self.errors.push(result.unwrap_err());
            }
        }
    }
}
