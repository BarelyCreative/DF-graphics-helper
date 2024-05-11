use egui::{Context, Sense, Ui, Key, Modifiers, KeyboardShortcut, Stroke, TextureHandle};
use egui_plot::{GridInput, GridMark, Plot, PlotImage, PlotPoint, Polygon};
use convert_case::{Case, Casing};
use rfd;
use std::path::PathBuf;
use std::path;

use crate::PADDING;
use super::error;
use crate::{Graphics, TilePage, Tile, CreatureFile, Creature, LayerSet, LayerGroup, Layer, SimpleLayer, Condition, State};
use error::{DFGHError, Result, error_window};


#[derive(Debug, Default, Clone, Copy)]
pub enum MainWindow {
    #[default]
    DefaultMenu,
    TilePageDefaultMenu,
    TilePageMenu,
    TileMenu,
    CreatureDefaultMenu,
    CreatureFileMenu,
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
    TilePage(TilePage),
    Tile(Tile),
    CreatureFile(CreatureFile),
    Creature(Creature),
    LayerSet(LayerSet),
    LayerGroup(LayerGroup),
    Layer(Layer),
    SimpleLayer(SimpleLayer),
    Condition(Condition),
}
impl From<TilePage> for ContextData {
    fn from(value: TilePage) -> Self {
        ContextData::TilePage(value)
    }
}
impl From<Tile> for ContextData {
    fn from(value: Tile) -> Self {
        ContextData::Tile(value)
    }
}
impl From<CreatureFile> for ContextData {
    fn from(value: CreatureFile) -> Self {
        ContextData::CreatureFile(value)
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
            MainWindow::DefaultMenu => ContextData::CreatureFile(CreatureFile::new()),
            MainWindow::TilePageDefaultMenu => ContextData::TilePage(TilePage::new()),
            MainWindow::TilePageMenu => ContextData::TilePage(TilePage::new()),
            MainWindow::TileMenu => ContextData::Tile(Tile::new()),
            MainWindow::CreatureDefaultMenu => ContextData::Creature(Creature::new()),
            MainWindow::CreatureFileMenu => ContextData::CreatureFile(CreatureFile::new()),
            MainWindow::CreatureMenu => ContextData::Creature(Creature::new()),
            MainWindow::LayerGroupMenu => ContextData::LayerGroup(LayerGroup::new()),
            MainWindow::LayerSetMenu => ContextData::LayerSet(LayerSet::Empty),
            MainWindow::LayerMenu => ContextData::Layer(Layer::new()),
            MainWindow::ConditionMenu => ContextData::Condition(Condition::Default),
            MainWindow::ReferenceMenu => ContextData::None,
        }
    }
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
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GraphicsIndices {
    tile_page_index: usize,
    tile_index: usize,
    creature_file_index: usize,
    creature_index: usize,
    layer_set_index: usize,
    layer_group_index: usize,
    layer_index: usize,
    condition_index: usize,
}
impl GraphicsIndices {
    fn new() -> GraphicsIndices {
        Self {
            tile_page_index: 0,
            tile_index: 0,
            creature_file_index: 0,
            creature_index: 0,
            layer_set_index: 0,
            layer_group_index: 0,
            layer_index: 0,
            condition_index: 0,
        }
    }
}
impl From<[usize; 8]> for GraphicsIndices {
    fn from(index_array: [usize; 8]) -> Self {
        GraphicsIndices {
            tile_page_index:        index_array[0],
            tile_index:             index_array[1],
            creature_file_index:    index_array[2],
            creature_index:         index_array[3],
            layer_set_index:        index_array[4],
            layer_group_index:      index_array[5],
            layer_index:            index_array[6],
            condition_index:        index_array[7]
        }
    }
}

pub struct DFGraphicsHelper {
    pub main_window: MainWindow,
    loaded_graphics: Graphics,
    pub indices: GraphicsIndices,
    path: std::path::PathBuf,
    texture_file_name: String,
    texture: Option<TextureHandle>,
    preview_image: bool,
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
            texture_file_name: String::new(),
            texture: None,
            preview_image: true,
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
                let export_result = self.loaded_graphics.display(&path);
                if export_result.is_err() {
                    self.exception = export_result.unwrap_err();
                };
            }
        } else {
            let export_result = self.loaded_graphics.display(&self.path);
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
                    ContextData::TilePage(_) | ContextData::Tile(_) => {
                        if ui.button("Tile Page").clicked() {
                            ui.close_menu();
                            let data = ContextData::from(TilePage::new());
                            inner_action = Action::Insert(data);
                        } else if ui.button("Tile").clicked() {
                            ui.close_menu();
                            let data = ContextData::from(Tile::new());
                            inner_action = Action::Insert(data);
                        }
                    },
                    ContextData::CreatureFile(_) |
                    ContextData::Creature(_) |
                    ContextData::LayerSet(_) |
                    ContextData::LayerGroup(_) |
                    ContextData::Layer(_) |
                    ContextData::SimpleLayer(_) |
                    ContextData::Condition(_) => {
                        if ui.button("Creature File").clicked() {
                            ui.close_menu();
                            let data = ContextData::from(CreatureFile::new());
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
                            let data = ContextData::from(LayerSet::Layered(State::Default, vec![LayerGroup::new()]));
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
            ContextData::TilePage(tile_page) => {
                let tps = &mut graphics.tile_pages;
                if indices.tile_page_index < tps.len() {
                    tps.insert(indices.tile_page_index, tile_page.clone());//checked
                    self.main_window = MainWindow::TilePageMenu;
                }
            },
            ContextData::Tile(tile) => {
                let ts = &mut graphics.tile_pages
                    .get_mut(indices.tile_page_index)
                    .ok_or(DFGHError::IndexError)?
                    .tiles;
                if indices.tile_index < ts.len() {
                    ts.insert(indices.tile_index, tile.clone());//checked
                    self.main_window = MainWindow::TileMenu;
                }
            },
            ContextData::CreatureFile(creature_file) => {
                let cfs = &mut graphics.creature_files;
                if indices.creature_file_index < cfs.len() {
                    cfs.insert(indices.creature_file_index, creature_file.clone());//checked
                    self.main_window = MainWindow::CreatureFileMenu;
                }
            },
            ContextData::Creature(creature) => {
                let cs = &mut graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .creatures;
                if indices.creature_index < cs.len() {
                    cs.insert(indices.creature_index, creature.clone());//checked
                    self.main_window = MainWindow::CreatureMenu;
                }
            },
            ContextData::LayerSet(layer_set) => {
                let lss = &mut graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .creatures
                    .get_mut(indices.creature_index)
                    .ok_or(DFGHError::IndexError)?
                    .graphics_type;
                if indices.layer_set_index < lss.len() {
                    lss.insert(indices.layer_set_index, layer_set.clone());//checked
                    self.main_window = MainWindow::LayerSetMenu;
                }
            },
            ContextData::LayerGroup(layer_group) => {
                if let LayerSet::Layered(_, layer_groups) = graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .creatures
                    .get_mut(indices.creature_index)
                    .ok_or(DFGHError::IndexError)?
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .ok_or(DFGHError::IndexError)? {
                    if indices.layer_group_index < layer_groups.len() {
                        layer_groups.insert(indices.layer_group_index, layer_group.clone());//checked
                        self.main_window = MainWindow::LayerGroupMenu;
                    }
                }
            },
            ContextData::SimpleLayer(simple_layer) => {
                if let LayerSet::Simple(simple_layers) |
                    LayerSet::Statue(simple_layers) = 
                    graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .creatures
                    .get_mut(indices.creature_index)
                    .ok_or(DFGHError::IndexError)?
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .ok_or(DFGHError::IndexError)? {
                    if indices.layer_index < simple_layers.len() {
                        simple_layers.insert(indices.layer_index, simple_layer.clone());//checked
                        self.main_window = MainWindow::LayerMenu;
                    }
                }
            },
            ContextData::Layer(layer) => {
                if let LayerSet::Layered(_, layer_groups) = graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .creatures
                    .get_mut(indices.creature_index)
                    .ok_or(DFGHError::IndexError)?
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .ok_or(DFGHError::IndexError)? {
                    let ls = &mut layer_groups
                        .get_mut(indices.layer_group_index)
                        .ok_or(DFGHError::IndexError)?
                        .layers;
                    if indices.layer_index < ls.len() {
                        ls.insert(indices.layer_index, layer.clone());//checked
                        self.main_window = MainWindow::LayerMenu;
                    }
                }
            },
            ContextData::Condition(condition) => {
                if let LayerSet::Layered(_, layer_groups) = graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .creatures
                    .get_mut(indices.creature_index)
                    .ok_or(DFGHError::IndexError)?
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .ok_or(DFGHError::IndexError)? {
                    let cs = &mut layer_groups
                        .get_mut(indices.layer_group_index)
                        .ok_or(DFGHError::IndexError)?
                        .layers
                        .get_mut(indices.layer_index)
                        .ok_or(DFGHError::IndexError)?
                        .conditions;
                    if indices.condition_index < cs.len() {
                        cs.insert(indices.condition_index, condition.clone());//checked
                        self.main_window = MainWindow::ConditionMenu;
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
            ContextData::TilePage(_) => {
                let tps = &mut graphics.tile_pages;
                if indices.tile_page_index < tps.len() {
                    tps.remove(indices.tile_page_index);//checked
                    if indices.tile_page_index >=1 {
                        indices.tile_page_index -= 1;
                    } else {
                        self.main_window = MainWindow::TilePageDefaultMenu;
                    }
                }
            },
            ContextData::Tile(_) => {
                let ts = &mut graphics.tile_pages
                    .get_mut(indices.tile_page_index)
                    .ok_or(DFGHError::IndexError)?
                    .tiles;
                if indices.tile_index < ts.len() {
                    ts.remove(indices.tile_index);//checked
                    if indices.tile_index >=1 {
                        indices.tile_index -= 1;
                    } else {
                        self.main_window = MainWindow::TilePageMenu;
                    }
                }
            },
            ContextData::CreatureFile(_) => {
                let cfs = &mut graphics.creature_files;
                if indices.creature_file_index < cfs.len() {
                    cfs.remove(indices.creature_file_index);//checked
                    if indices.creature_file_index >=1 {
                        indices.creature_file_index -= 1;
                    } else {
                        self.main_window = MainWindow::CreatureDefaultMenu;
                    }
                }
            },
            ContextData::Creature(_) => {
                let cs = &mut graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .creatures;
                if indices.creature_index < cs.len() {
                    cs.remove(indices.creature_index);//checked
                    if indices.creature_index >=1 {
                        indices.creature_index -= 1;
                    } else {
                        self.main_window = MainWindow::CreatureFileMenu;
                    }
                }
            },
            ContextData::LayerSet(_) => {
                let lss = &mut graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .creatures
                    .get_mut(indices.creature_index)
                    .ok_or(DFGHError::IndexError)?
                    .graphics_type;
                if indices.layer_set_index < lss.len() {
                    lss.remove(indices.layer_set_index);//checked
                    if indices.layer_set_index >=1 {
                        indices.layer_set_index -= 1;
                    } else {
                        self.main_window = MainWindow::CreatureMenu;
                    }
                }
            },
            ContextData::LayerGroup(_) => {
                if let LayerSet::Layered(_, layer_groups) = graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .creatures
                    .get_mut(indices.creature_index)
                    .ok_or(DFGHError::IndexError)?
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .ok_or(DFGHError::IndexError)? {
                    if indices.layer_group_index < layer_groups.len() {
                        layer_groups.remove(indices.layer_group_index);//checked
                        if indices.layer_group_index >=1 {
                            indices.layer_group_index -= 1;
                        } else {
                            self.main_window = MainWindow::LayerSetMenu;
                        }
                    }
                }
            },
            ContextData::SimpleLayer(_) => {
                if let LayerSet::Simple(simple_layers) |
                    LayerSet::Statue(simple_layers) = 
                    graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .creatures
                    .get_mut(indices.creature_index)
                    .ok_or(DFGHError::IndexError)?
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .ok_or(DFGHError::IndexError)? {
                    if indices.layer_index < simple_layers.len() {
                        simple_layers.remove(indices.layer_index);//checked
                        if indices.layer_index >=1 {
                            indices.layer_index -= 1;
                        } else {
                            self.main_window = MainWindow::CreatureMenu;
                        }
                    }
                }
            },
            ContextData::Layer(_) => {
                if let LayerSet::Layered(_, layer_groups) = graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .creatures
                    .get_mut(indices.creature_index)
                    .ok_or(DFGHError::IndexError)?
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .ok_or(DFGHError::IndexError)? {
                    let ls = &mut layer_groups
                        .get_mut(indices.layer_group_index)
                        .ok_or(DFGHError::IndexError)?
                        .layers;
                    if indices.layer_index < ls.len() {
                        ls.remove(indices.layer_index);//checked
                        if indices.layer_index >=1 {
                            indices.layer_index -= 1;
                        } else {
                            self.main_window = MainWindow::LayerGroupMenu;
                        }
                    }
                }
            },
            ContextData::Condition(_) => {
                if let LayerSet::Layered(_, layer_groups) = graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .ok_or(DFGHError::IndexError)?
                    .creatures
                    .get_mut(indices.creature_index)
                    .ok_or(DFGHError::IndexError)?
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .ok_or(DFGHError::IndexError)? {
                    let cs = &mut layer_groups
                        .get_mut(indices.layer_group_index)
                        .ok_or(DFGHError::IndexError)?
                        .layers
                        .get_mut(indices.layer_index)
                        .ok_or(DFGHError::IndexError)?
                        .conditions;
                    if indices.condition_index < cs.len() {
                        cs.remove(indices.condition_index);//checked
                        if indices.condition_index >=1 {
                            indices.condition_index -= 1;
                        } else {
                            self.main_window = MainWindow::LayerMenu;
                        }
                    }
                }
            },
            ContextData::None => {},
        }
        self.action = Action::None;
        Ok(())
    }

    fn main_tree(&mut self, ui: &mut Ui, ctx: &Context) {
        let graphics = &mut self.loaded_graphics;

        if ui
            .add(egui::Label::new("Tile Pages").sense(Sense::click()))
            .clicked()
        {
            self.main_window = MainWindow::TilePageDefaultMenu;
        };
        for (i_tile_page, tile_page) in graphics.tile_pages.iter_mut().enumerate() {
            let id_t = ui.make_persistent_id(
                format!("tile_page{}", i_tile_page)
            );
            egui::collapsing_header::CollapsingState::load_with_default_open(
                ctx,
                id_t,
                true,
            )
            .show_header(ui, |ui| {
                let tilepage_response = ui.add(egui::Label::new(
                    format!("Tile Page: {}", &tile_page.name))
                    .sense(Sense::click()));
                if tilepage_response.clicked() {
                    self.indices = [i_tile_page, 0, 0, 0, 0, 0, 0, 0].into();
                    self.main_window = MainWindow::TilePageMenu;
                }
                tilepage_response.context_menu(|ui| {
                    self.indices = [i_tile_page, 0, 0, 0, 0, 0, 0, 0].into();
                    self.action = Self::context(ui, ContextData::from(tile_page.clone()));
                });
            })
            .body(|ui| {
                for (i_tile, tile) in tile_page.tiles.iter_mut().enumerate() {
                    let tile_response = ui.add(egui::Label::new(
                        format!("{}", &tile.name))
                        .wrap(false)
                        .sense(Sense::click()));
                    if tile_response.clicked() {
                        self.indices = [i_tile_page, i_tile, 0, 0, 0, 0, 0, 0].into();
                        self.main_window = MainWindow::TileMenu;
                    }
                    tile_response.context_menu(|ui| {
                        self.indices = [i_tile_page, i_tile, 0, 0, 0, 0, 0, 0].into();
                        self.action = Self::context(ui, ContextData::from(tile.clone()));
                    });
                }
            });
        }

        ui.separator();
        if ui
            .add(egui::Label::new("Creature Graphics").sense(Sense::click()))
            .clicked()
        {
            self.main_window = MainWindow::CreatureDefaultMenu;
        };
        for (i_file, creature_file) in self.loaded_graphics.creature_files.iter_mut().enumerate() {
            let id_cf = ui.make_persistent_id(
                format!("creature_file{}",
                i_file)
            );
            egui::collapsing_header::CollapsingState::load_with_default_open(
                ctx,
                id_cf,
                true,
            )
            .show_header(ui, |ui| {
                let creature_file_response = ui.add(egui::Label::new(
                    format!("File: {}", &creature_file.name))
                    .sense(Sense::click()));
                if creature_file_response.clicked() {
                    self.indices = [0, 0, i_file, 0, 0, 0, 0, 0].into();
                    self.main_window = MainWindow::CreatureFileMenu;
                }
                creature_file_response.context_menu(|ui| {
                    self.indices = [0, 0, i_file, 0, 0, 0, 0, 0].into();
                    self.action = Self::context(ui, ContextData::from(creature_file.clone()));
                });
            })
            .body(|ui| {
                for (i_creature, creature) in creature_file.creatures.iter_mut().enumerate() {
                    let id_c = ui.make_persistent_id(
                        format!("creature{}{}",
                        i_file, i_creature)
                    );
                    egui::collapsing_header::CollapsingState::load_with_default_open(
                        ctx,
                        id_c,
                        true,
                    )
                    .show_header(ui, |ui| {
                        let creature_response = ui.add(egui::Label::new(
                            format!("{}", &creature.name))
                            .sense(Sense::click()));
                        if creature_response.clicked() {
                            self.indices = [0, 0, i_file, i_creature, 0, 0, 0, 0].into();
                            self.main_window = MainWindow::CreatureMenu;
                        }
                        creature_response.context_menu(|ui| {
                            self.indices = [0, 0, i_file, i_creature, 0, 0, 0, 0].into();
                            self.action = Self::context(ui, ContextData::from(creature.clone()));
                        });
                    })
                    .body(|ui| {
                        for (i_layer_set, layer_set) in creature.graphics_type.iter_mut().enumerate() {
                            match layer_set {
                                LayerSet::Empty => {
                                    let layerset_response = ui.add(egui::Label::new("(empty)")
                                        .sense(Sense::click()));
                                    if layerset_response.clicked() {
                                        self.indices = [0, 0, i_file, i_creature, i_layer_set, 0, 0, 0].into();
                                        self.main_window = MainWindow::LayerSetMenu;
                                    }
                                    layerset_response.context_menu(|ui| {
                                        self.indices = [0, 0, i_file, i_creature, i_layer_set, 0, 0, 0].into();
                                        self.action = Self::context(ui, ContextData::from(layer_set.clone()));
                                    });
                                },
                                LayerSet::Layered(state, layer_groups) => {
                                    let id_ls = ui.make_persistent_id(
                                        format!("layer_set{}{}{}",
                                        i_file, i_creature, i_layer_set)
                                    );
                                    egui::collapsing_header::CollapsingState::load_with_default_open(ctx,
                                        id_ls,
                                        true)
                                        .show_header(ui, |ui|
                                        {
                                            let layerset_response = ui.add(egui::Label::new(
                                                format!("{}", state.name()))
                                                .sense(Sense::click()));
                                            if layerset_response.clicked() {
                                                self.indices = [0, 0, i_file, i_creature, i_layer_set, 0, 0, 0].into();
                                                self.main_window = MainWindow::LayerSetMenu;
                                            }
                                            layerset_response.context_menu(|ui| {
                                                self.indices = [0, 0, i_file, i_creature, i_layer_set, 0, 0, 0].into();
                                                self.action = Self::context(ui, ContextData::from(LayerSet::Layered(state.clone(), layer_groups.clone())));
                                            });
                                    })
                                        .body(|ui|
                                        {
                                        for (i_layer_group, layer_group) in layer_groups.iter_mut().enumerate() {
                                            let id_lg = ui.make_persistent_id(
                                                format!("layer_group{}{}{}{}",
                                                i_file, i_creature, i_layer_set, i_layer_group)
                                            );
                                            egui::collapsing_header::CollapsingState::load_with_default_open(ctx,
                                                id_lg,
                                                false)
                                                .show_header(ui, |ui|
                                                {
                                                    let layergroup_response = ui.add(egui::Label::new(
                                                        format!("{}", &layer_group.name))
                                                        .sense(Sense::click()));
                                                    if layergroup_response.clicked() {
                                                        self.indices = [0, 0, i_file, i_creature, i_layer_set, i_layer_group, 0, 0].into();
                                                        self.main_window = MainWindow::LayerGroupMenu;
                                                    }
                                                    layergroup_response.context_menu(|ui| {
                                                        self.indices = [0, 0, i_file, i_creature, i_layer_set, i_layer_group, 0, 0].into();
                                                        self.action = Self::context(ui, ContextData::from(layer_group.clone()));
                                                    });
                                                })
                                                .body(|ui|
                                                {
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
                                                        for (i_condition, condition) in layer.conditions.iter_mut().enumerate() {
                                                            let condition_response = ui.add(egui::Label::new(
                                                                format!("\t{}", condition.name()))
                                                                .wrap(false)
                                                                .sense(Sense::click()));
                                                            if condition_response.clicked() {
                                                                self.indices = [0, 0, i_file, i_creature, i_layer_set, i_layer_group, i_layer, i_condition].into();
                                                                self.main_window = MainWindow::ConditionMenu;
                                                            }
                                                            condition_response.context_menu(|ui| {
                                                                self.indices = [0, 0, i_file, i_creature, i_layer_set, i_layer_group, i_layer, i_condition].into();
                                                                self.action = Self::context(ui, ContextData::from(condition.clone()));
                                                            });
                                                        }
                                                    });
                                                }
                                            });
                                        }
                                    });
                                },
                                LayerSet::Simple(simple_layers) => {
                                    for (i_layer, simple_layer) in simple_layers.iter_mut().enumerate() {
                                        let condition_response = ui.add(egui::Label::new(
                                        if let Some(sub_state) = &simple_layer.sub_state {
                                                format!("\t{} + {}",
                                                simple_layer.state.name(),
                                                sub_state.name())
                                            } else {
                                                format!("\t{}",
                                                simple_layer.state.name())
                                            })
                                            .wrap(false)
                                            .sense(Sense::click()));
                                        if condition_response.clicked() {
                                            self.indices = [0, 0, i_file, i_creature, i_layer_set, 0, i_layer, 0].into();
                                            self.main_window = MainWindow::LayerMenu;
                                        }
                                        condition_response.context_menu(|ui| {
                                            self.indices = [0, 0, i_file, i_creature, i_layer_set, 0, i_layer, 0].into();
                                            self.action = Self::context(ui, ContextData::from(simple_layer.clone()));
                                        });
                                    }
                                },
                                LayerSet::Statue(simple_layers) => {
                                    for (i_layer, simple_layer) in simple_layers.iter_mut().enumerate() {
                                        let condition_response = ui.add(egui::Label::new(
                                        if let Some(sub_state) = &simple_layer.sub_state {
                                                format!("\t{} + {}",
                                                simple_layer.state.name(),
                                                sub_state.name())
                                            } else {
                                                format!("\t{}",
                                                simple_layer.state.name())
                                            })
                                            .wrap(false)
                                            .sense(Sense::click()));
                                        if condition_response.clicked() {
                                            self.indices = [0, 0, i_file, i_creature, i_layer_set, 0, i_layer, 0].into();
                                            self.main_window = MainWindow::LayerMenu;
                                        }
                                        condition_response.context_menu(|ui| {
                                            self.indices = [0, 0, i_file, i_creature, i_layer_set, 0, i_layer, 0].into();
                                            self.action = Self::context(ui, ContextData::from(simple_layer.clone()));
                                        });
                                    }
                                },
                            }
                        }
                    });
                }
            });
        }

        ui.separator();
        if ui
            .add(egui::Label::new("References").sense(Sense::click()))
            .clicked()
        {
            self.main_window = MainWindow::ReferenceMenu;
        };
    }

    fn default_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.label("Welcome!");
        ui.separator();

        ui.add_space(PADDING);
        ui.hyperlink_to(
            "DF Graphics Helper on GitHub",
            "https://github.com/BarelyCreative/DF-graphics-helper/tree/main",
        );
        Ok(())
    }

    fn tile_page_default_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.label("Tile Page Menu");
        ui.separator();

        if ui.small_button("New Tile Page").clicked() {
            self.action = Action::Insert(ContextData::TilePage(TilePage::new()));
        }
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

        if self.loaded_graphics.tile_pages.is_empty() {
            self.action = Action::Insert(ContextData::TilePage(TilePage::new()));
        } else {
            let tile_page = self
                .loaded_graphics
                .tile_pages
                .get_mut(indices.tile_page_index)
                .ok_or(DFGHError::IndexError)?;

            ui.separator();
            ui.text_edit_singleline(&mut tile_page.name);
            ui.add_space(PADDING);

            if ui.button("New Tile").clicked() {
                self.action = Action::Insert(ContextData::Tile(Tile::new()));
            }
        }
        Ok(())
    }

    fn tile_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Tile Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::Tile(Tile::new()));
            }
        });
        
        let indices = &mut self.indices;

        let tiles = &mut self
            .loaded_graphics
            .tile_pages
            .get_mut(indices.tile_page_index)
            .ok_or(DFGHError::IndexError)?
            .tiles;

        if tiles.is_empty() {
            if ui.small_button("Create Tile").clicked() {
                self.action = Action::Insert(ContextData::Tile(Tile::new()));
            }
        } else {
            let tile = tiles
                .get_mut(indices.tile_index)
                .ok_or(DFGHError::IndexError)?;
            let file_name = tile.filename.clone();

            tile.tile_menu(ui);

            ui.add_space(PADDING);
            self.preview_image(ui, file_name, None)?;
        }
        Ok(())
    }

    fn creature_default_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.label("Creature File Menu");
        ui.separator();

        if ui.small_button("New Creature File").clicked() {
            self.action = Action::Insert(ContextData::CreatureFile(CreatureFile::new()));
        }
        Ok(())
    }

    fn creature_file_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Creature File Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::CreatureFile(CreatureFile::new()));
            }
        });
        
        let indices = &mut self.indices;

        if self.loaded_graphics.creature_files.is_empty() {
            self.main_window = MainWindow::CreatureDefaultMenu;
        } else {
            let creature_file = self
                .loaded_graphics
                .creature_files
                .get_mut(indices.creature_file_index)
                .ok_or(DFGHError::IndexError)?;

            ui.separator();
            ui.text_edit_singleline(&mut creature_file.name);
            ui.add_space(PADDING);

            if ui.button("New Creature").clicked() {
                self.action = Action::Insert(ContextData::Creature(Creature::new()));
            }
        }
        Ok(())
    }

    fn creature_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Creature Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::Creature(Creature::new()));
            }
        });

        let indices = &mut self.indices;

        let creatures = &mut self
            .loaded_graphics
            .creature_files
            .get_mut(indices.creature_file_index)
            .ok_or(DFGHError::IndexError)?
            .creatures;
        
        if creatures.is_empty() {
            if ui.small_button("Create Creature").clicked() {
                self.action = Action::Insert(ContextData::Creature(Creature::new()));
            }
        } else {
            let creature = creatures
                .get_mut(indices.creature_index)
                .ok_or(DFGHError::IndexError)?;

            creature.creature_menu(ui);
        }
        Ok(())
    }

    fn layer_set_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Layer Set Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::LayerSet(LayerSet::Simple(Vec::new())));
            }
        });

        let indices = &mut self.indices;

        let layer_sets = &mut self
            .loaded_graphics
            .creature_files
            .get_mut(indices.creature_file_index)
            .ok_or(DFGHError::IndexError)?
            .creatures
            .get_mut(indices.creature_index)
            .ok_or(DFGHError::IndexError)?
            .graphics_type;
        
        if layer_sets.is_empty() {
            if ui.small_button("Create Layer Set").clicked() {
                self.action = Action::Insert(ContextData::LayerSet(LayerSet::default()));
            }
        } else {
            let layer_set = layer_sets
                .get_mut(indices.layer_set_index)
                .ok_or(DFGHError::IndexError)?;

            layer_set.layer_set_menu(ui);
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

        let graphics_type = self
            .loaded_graphics
            .creature_files
            .get_mut(indices.creature_file_index)
            .ok_or(DFGHError::IndexError)?
            .creatures
            .get_mut(indices.creature_index)
            .ok_or(DFGHError::IndexError)?
            .graphics_type
            .get_mut(indices.layer_set_index)
            .ok_or(DFGHError::IndexError)?;

        if let LayerSet::Layered(_, layer_groups) = graphics_type {
            if layer_groups.is_empty() {
                if ui.small_button("Create Layer Group").clicked() {
                    self.action = Action::Insert(ContextData::LayerGroup(LayerGroup::new()));
                }
            } else {
                let layer_group = layer_groups
                    .get_mut(indices.layer_group_index)
                    .ok_or(DFGHError::IndexError)?;

                layer_group.layer_group_menu(ui);
            }
        }
        Ok(())
    }

    fn layer_menu(&mut self, ui: &mut Ui) -> Result<()> {
        let tile_info = self.tile_info();

        let cursor_coords = self.cursor_coords;
        
        let indices = &mut self.indices;

        let layer_groups = self
            .loaded_graphics
            .creature_files
            .get_mut(indices.creature_file_index)
            .ok_or(DFGHError::IndexError)?
            .creatures
            .get_mut(indices.creature_index)
            .ok_or(DFGHError::IndexError)?
            .graphics_type
            .get_mut(indices.layer_set_index)
            .ok_or(DFGHError::IndexError)?;

        let tiles: Vec<&Tile> = self
            .loaded_graphics
            .tile_pages
            .iter()
            .flat_map(|tp| tp.tiles.iter())
            .collect();

        match layer_groups {
            LayerSet::Simple(simple_layers) => {
                if simple_layers.is_empty() {
                    //if there are no layers defined show create layer button only
                    ui.label("Simple Layer Menu");
                    ui.separator();
                    if ui.small_button("Create Layer").clicked() {
                        self.action = Action::Insert(ContextData::SimpleLayer(SimpleLayer::new()));
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Layer Menu");
                        if ui.button("Delete").clicked() {
                            self.action = Action::Delete(ContextData::SimpleLayer(SimpleLayer::new()));
                        }
                    });

                    let simple_layer = simple_layers
                        .get_mut(indices.layer_index)
                        .ok_or(DFGHError::IndexError)?;

                    if let Some(coords) = cursor_coords {
                        simple_layer.coords = coords;
                    }

                    simple_layer.layer_menu(ui, tile_info);

                    ui.add_space(PADDING);
                    let mut file_name = String::new();
                    for tile in tiles {
                        if tile.name.to_case(Case::UpperSnake).eq(&simple_layer.tile.to_case(Case::UpperSnake)) {
                            file_name = tile.filename.clone();
                            break;
                        }
                    }
                    let rect = Some([simple_layer.coords, simple_layer.large_coords.unwrap_or_else(|| [0, 0])]);
                    self.preview_image(ui, file_name, rect)?;
                }
            },
            LayerSet::Statue(simple_layers) => {
                if simple_layers.is_empty() {
                    //if there are no layers defined show create layer button only
                    ui.label("Statue Layer Menu");
                    ui.separator();
                    if ui.small_button("Create Layer").clicked() {
                        self.action = Action::Insert(ContextData::SimpleLayer(SimpleLayer::new()));
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Layer Menu");
                        if ui.button("Delete").clicked() {
                            self.action = Action::Delete(ContextData::SimpleLayer(SimpleLayer::new()));
                        }
                    });
                    
                    let simple_layer = simple_layers
                        .get_mut(indices.layer_index)
                        .ok_or(DFGHError::IndexError)?;

                    if let Some(coords) = cursor_coords {
                        simple_layer.coords = coords;
                    }

                    simple_layer.statue_layer_menu(ui, tile_info);

                    ui.add_space(PADDING);
                    let mut file_name = String::new();
                    for tile in tiles {
                        if tile.name.to_case(Case::UpperSnake).eq(&simple_layer.tile.to_case(Case::UpperSnake)) {
                            file_name = tile.filename.clone();
                            break;
                        }
                    }
                    let rect = Some([simple_layer.coords, simple_layer.large_coords.unwrap_or_else(|| [0, 0])]);
                    self.preview_image(ui, file_name, rect)?;
                }
            },
            LayerSet::Layered(_, layer_groups) => {
                let layers = 
                    &mut layer_groups
                    .get_mut(indices.layer_group_index)
                    .ok_or(DFGHError::IndexError)?
                    .layers;

                if layers.is_empty() {
                    //if there are no layers defined show create layer button only
                    ui.label("Layer Menu");
                    ui.separator();
                    if ui.small_button("Create Layer").clicked() {
                        self.action = Action::Insert(ContextData::Layer(Layer::new()));
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Layer Menu");
                        if ui.button("Delete").clicked() {
                            self.action = Action::Delete(ContextData::Layer(Layer::new()));
                        }
                    });

                    let layer = layers
                        .get_mut(indices.layer_index)
                        .ok_or(DFGHError::IndexError)?;

                    if let Some(coords) = cursor_coords {
                        layer.coords = coords;
                    }

                    layer.layer_menu(ui, tile_info);

                    ui.add_space(PADDING);
                    let mut file_name = String::new();
                    for tile in tiles {
                        if tile.name.to_case(Case::UpperSnake).eq(&layer.tile.to_case(Case::UpperSnake)) {
                            file_name = tile.filename.clone();
                            break;
                        }
                    }
                    let rect = Some([layer.coords, layer.large_coords.unwrap_or_else(|| [0, 0])]);
                    self.preview_image(ui, file_name, rect)?;
                }

            },
            LayerSet::Empty => {
                ui.horizontal(|ui| {
                    ui.label("Empty Layer Menu");
                    if ui.button("Delete").clicked() {
                        self.action = Action::Delete(ContextData::Layer(Layer::new()));
                    }
                });
            },
        }
        Ok(())
    }

    fn condition_menu(&mut self, ui: &mut Ui) -> Result<()> {
        ui.horizontal(|ui| {
            ui.label("Condition Menu");
            if ui.button("Delete").clicked() {
                self.action = Action::Delete(ContextData::Condition(Condition::default()));
            }
        });

        let tile_info = self.tile_info();
        
        let indices = &mut self.indices;

        let graphics_type = self
            .loaded_graphics
            .creature_files
            .get_mut(indices.creature_file_index)
            .ok_or(DFGHError::IndexError)?
            .creatures
            .get_mut(indices.creature_index)
            .ok_or(DFGHError::IndexError)?
            .graphics_type
            .get_mut(indices.layer_set_index)
            .ok_or(DFGHError::IndexError)?;

        if let LayerSet::Layered(_, layergroups) = graphics_type {
            let conditions = &mut layergroups
                .get_mut(indices.layer_group_index)
                .ok_or(DFGHError::IndexError)?
                .layers
                .get_mut(indices.layer_index)
                .ok_or(DFGHError::IndexError)?
                .conditions;

            if conditions.is_empty() {
                if ui.small_button("New condition").clicked() {
                    self.action = Action::Insert(ContextData::Condition(Condition::default()));
                }
            } else {
                let condition = conditions
                    .get_mut(indices.condition_index)
                    .ok_or(DFGHError::IndexError)?;

                ui.separator();
    
                condition.condition_menu(ui, tile_info);
            }
        }
        Ok(())
    }

    fn preview_image(&mut self, ui: &mut Ui, file_name: String, rectangle: Option<[[u32; 2]; 2]>) -> Result<()> {
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.preview_image, "View Image"); //determine if preview image is desired
            if ui.button("Refresh").clicked() {
                //clear image if stale
                self.texture = None;
            }
        });

        if self.preview_image && self.texture.is_some() {
            //display texture once loaded
            let texture: &TextureHandle = self.texture.as_ref().expect("reference to texture should not be empty");//checked
            let size = texture.size_vec2();

            

            let image =
                PlotImage::new(texture, PlotPoint::new(size[0] / 2.0, size[1] / -2.0), size);

            let x_fmt = 
            |grid_mk: GridMark, _max_char: usize, _range: &core::ops::RangeInclusive<f64> | {
                let x = grid_mk.value;
                if x < 0.0 {
                    // No labels outside value bounds
                    String::new()
                } else {
                    // Tiles
                    format!("{}", (x as f64 / 32.0).floor())
                }
            };
            let y_fmt = 
            |grid_mk: GridMark, _max_char: usize, _range: &core::ops::RangeInclusive<f64> | {
                let y = grid_mk.value;
                if y > 0.0 {
                    // No labels outside value bounds
                    String::new()
                } else {
                    // Tiles
                    format!("{}", (y as f64 / -32.0).floor())
                }
            };
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

            let plot = Plot::new("image_preview")
                .auto_bounds([true, true].into())
                .data_aspect(1.0)
                .show_background(true)
                .allow_boxed_zoom(false)
                .clamp_grid(true)
                .min_size(egui::vec2(100.0, 400.0))
                .set_margin_fraction(egui::vec2(0.005, 0.005))
                .x_axis_formatter(x_fmt) //todo
                .y_axis_formatter(y_fmt) //todo
                .x_grid_spacer(grid_fmt)
                .y_grid_spacer(grid_fmt)
                .label_formatter(label_fmt);
            plot.show(ui, |plot_ui| {
                plot_ui.image(image.name("Image"));
                if let Some(rect) = rectangle {
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
            });

            if self.texture_file_name.ne(&file_name) {
                self.texture = None;
            }
        } else if self.preview_image && self.texture.is_none() {
            //load texture from path
            let image_path = self.path
                .join("graphics")
                .join("images")
                .join(format!("{}.png", &file_name));

            if image_path.exists() {
                let dyn_image = image::open(image_path)?;
                let size = [dyn_image.width() as _, dyn_image.height() as _];
                let image = dyn_image.as_bytes();
                let rgba = egui::ColorImage::from_rgba_unmultiplied(size, image);

                self.loaded_graphics.tile_pages
                    .get_mut(self.indices.tile_page_index)
                    .ok_or(DFGHError::IndexError)?
                    .tiles
                    .get_mut(self.indices.tile_index)
                    .ok_or(DFGHError::IndexError)?
                    .image_size = [dyn_image.width(), dyn_image.height()];
                
                self.texture.get_or_insert_with(|| {
                    ui.ctx()
                        .load_texture("default_image", rgba, Default::default())
                });
                self.texture_file_name = file_name;
            }
        }
        
        Ok(())
    }

    fn tile_info(&self) -> Vec<(String, [u32; 2])> {
        let mut tile_info: Vec<(String, [u32; 2])> = self
            .loaded_graphics
            .tile_pages
            .iter()
            .flat_map(|tile_page| {
                tile_page.tiles.iter().map(|t| {
                    (t.name.clone(),
                    [t.image_size[0].checked_div(t.tile_size[0])
                    .unwrap_or_default()
                    .checked_sub(1)
                    .unwrap_or_default(),
                    t.image_size[1].checked_div(t.tile_size[1])
                    .unwrap_or_default()
                    .checked_sub(1)
                    .unwrap_or_default()])
                })
            })
            .collect();

        tile_info.sort();
        tile_info.dedup();
        
        tile_info
    }

    pub fn tile_read(tile_info: &Vec<(String, [u32; 2])>, name: &String) -> (Vec<String>, [u32; 2]) {
        let tile_names: Vec<String> = tile_info.iter().map(|ti| ti.0.clone()).collect();
        let max_coords: [u32; 2];
        if let Some(idx_name) = tile_names.iter().position(|n| n == name) {
            max_coords = tile_info[idx_name].1;
        } else {
            max_coords = [100, 100];
        }

        (tile_names, max_coords)
    }
}

impl eframe::App for DFGraphicsHelper {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top control panel").show(ctx, |ui| {
            //Draw File menu tab and internal items
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
            });
        });

        egui::SidePanel::new(egui::panel::Side::Left, "tree")
            .resizable(true)
            .show(ctx, |ui| {
                //Draw tree-style selection menu on left side
                egui::ScrollArea::both().show(ui, |ui| {
                    self.main_tree(ui, ctx)
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            //Draw main window by matching self.main_window
            egui::ScrollArea::horizontal()
                .show(ui, |ui| {
                let result;
                match self.main_window {
                    MainWindow::TilePageDefaultMenu =>  result = self.tile_page_default_menu(ui),
                    MainWindow::CreatureDefaultMenu =>  result = self.creature_default_menu(ui),
                    MainWindow::TilePageMenu =>         result = self.tile_page_menu(ui),
                    MainWindow::TileMenu =>             result = self.tile_menu(ui),
                    MainWindow::CreatureFileMenu =>     result = self.creature_file_menu(ui),
                    MainWindow::CreatureMenu =>         result = self.creature_menu(ui),
                    MainWindow::LayerSetMenu =>         result = self.layer_set_menu(ui),
                    MainWindow::LayerGroupMenu =>       result = self.layer_group_menu(ui),
                    MainWindow::LayerMenu =>            result = self.layer_menu(ui),
                    MainWindow::ConditionMenu =>        result = self.condition_menu(ui),
                    MainWindow::ReferenceMenu =>        result = self.default_menu(ui),
                    MainWindow::DefaultMenu =>          result = self.default_menu(ui),
                }
                if result.is_err() {
                    self.exception = result.unwrap_err();
                }
            });
        });

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

        {//Action handler
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
                Action::None => {},
            }
            if result.is_err() {
                self.exception = result.unwrap_err();
            }
        }

        if !matches!(self.exception, DFGHError::None) {
            error_window(self, ctx);
        }
    }
}
