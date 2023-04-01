use egui::plot::{Plot, PlotImage, PlotPoint};
use egui::{Context, Sense, Ui};
// use convert_case::{Case, Casing};
use rfd;
use std::io::prelude::*;
use std::{fs, io, path, vec};

const PADDING: f32 = 8.0;

#[derive(Clone, Default)]
struct Graphics {
    tilepages: Vec<TilePage>,
    creature_files: Vec<CreatureFile>,
}
impl Graphics {
    fn new() -> Graphics {
        Graphics {
            tilepages: vec![TilePage::new()],
            creature_files: vec![CreatureFile::new()],
        }
    }

    fn read_brackets(raw_line: &String) -> (bool, Vec<String>) {
        let brackets = raw_line.contains("[") && raw_line.contains("]");
        let line_vec: Vec<String>;
        if brackets {
            let clean_line = raw_line.replace("[", "").replace("]", "").trim().to_string();

            line_vec = clean_line.split(":").map(str::to_string).collect()
        } else {
            line_vec = Vec::new();
        }

        (brackets, line_vec) //retain first bracket to ignore commented-out lines
    }

    fn import(folder: path::PathBuf) -> Graphics {
        let mut tilepages: Vec<TilePage> = Vec::new();
        let mut creature_files: Vec<CreatureFile> = Vec::new();

        let paths = fs::read_dir(folder).expect("expects 0 or more valid file paths"); //read graphics directory

        for path in paths {
            let dir_entry = path.unwrap();
            let entry_name = dir_entry.file_name().into_string().expect("utf-8 string");//only supports utf-8 file paths

            if entry_name.ends_with(".txt") {
                if entry_name.starts_with("tile_page_") {
                    //read tile page file
                    let mut tilepage = TilePage::empty();
                    let mut tile = Tile::empty();

                    let f =
                        fs::File::open(dir_entry.path()).expect("failed to open tile page file");
                    
                    for raw_line_result in io::BufReader::new(f).lines() {
                        //read line-by-line
                        let raw_line = raw_line_result.expect("the for loop should always give a valid String");
                        let (brackets, line_vec) = Self::read_brackets(&raw_line);

                        if tilepage.name.is_empty() {
                            tilepage.name = raw_line.replace("tile_page_", "").trim().to_string();

                        } else if brackets && line_vec.len() > 0 {
                            if line_vec[0].eq("TILE_PAGE") {
                                if !tile.name.is_empty() {
                                    tilepage.tiles.push(tile.clone());
                                    tile = Tile::empty();
                                    tile.name.clear();
                                }
                                tile.name = line_vec[1].clone();

                            } else if line_vec[0].eq("FILE") {
                                tile.filename = line_vec[1].clone();
                                
                            } else if line_vec[0].eq("TILE_DIM") {
                                tile.tile_size = 
                                    [line_vec[1].parse().unwrap_or_default(),
                                    line_vec[2].parse().unwrap_or_default()];

                            } else if line_vec[0].eq("PAGE_DIM_PIXELS") {
                                tile.image_size = 
                                    [line_vec[1].parse().unwrap_or_default(),
                                    line_vec[2].parse().unwrap_or_default()];

                            }
                        }
                    }
                    tilepage.tiles.push(tile);
                    tilepages.push(tilepage);

                } else if entry_name.starts_with("graphics_creatures_") {
                    let mut creature_file = CreatureFile::empty();
                    let mut creature = Creature::empty();
                    let mut layer_set = LayerSet::default();
                    let mut layer_group = LayerGroup::empty();
                    let mut simple_layer = SimpleLayer::empty();
                    let mut layer = Layer::empty();
                    let mut condition = Condition::default();

                    let f =
                        fs::File::open(dir_entry.path()).expect("failed to open tile page file");

                    for raw_line_result in io::BufReader::new(f).lines() {

                        let raw_line = raw_line_result.unwrap();
                        let (brackets, mut line_vec) = Self::read_brackets(&raw_line);
                        // let line_vec = line_vec_as_is.drain_filter(|string| string.as_ref() == "AS_IS").collect();
                        line_vec.retain(|elem| elem != "AS_IS");//remove any AS_IS elements there are for creature files due to not palletization(?) v50.05

                        if creature_file.name.is_empty() {
                            //set creature file name
                            creature_file.name =
                                raw_line.replace("graphics_creatures_", "").trim().to_string();
                        } else if brackets && line_vec.len() > 0 {

                            dbg!(&raw_line);

                            match line_vec[0].as_str() {
                                "CREATURE_GRAPHICS" | "STATUE_CREATURE_GRAPHICS" | "LAYER_SET" => {
                                    match &mut layer_set {
                                        //write buffered creature/layer set before starting the new one
                                        LayerSet::Empty => {
                                            //nothing defined, there is nothing to push
                                        },
                                        LayerSet::Layered(_, layer_groups) => {
                                            //if a new creature graphics is encountered, then the previous one must be finished
                                            // => write everything to the vector that contains it
                                            if !layer_groups.is_empty() {
                                                layer.conditions.push(condition.clone());
                                                condition = Condition::default();
                                                layer_group.layers.push(layer.clone());
                                                layer = Layer::empty();
                                                layer_groups.push(layer_group.clone());
                                                layer_group = LayerGroup::empty();
                                                creature.graphics_type.push(layer_set.clone());
                                            }
                                            //layer_groups is a temporary variable
                                            //layer_set buffer written in match below
                                            dbg!("breakpt new creature/set");
                                            dbg!(&creature);
                                        },
                                        LayerSet::Simple(simple_layers) | LayerSet::Statue(simple_layers) => {
                                            if !simple_layers.is_empty() {
                                                simple_layers.push(simple_layer);
                                                simple_layer = SimpleLayer::empty();
                                                creature.graphics_type.push(layer_set.clone());
                                                creature_file.creatures.push(creature.clone());
                                                creature.graphics_type.clear();
                                            }
                                        },
                                        // _ => {panic!()}
                                    }
                                    match line_vec[0].as_str() {
                                        //set up the buffered graphics according to the current line
                                        "CREATURE_GRAPHICS" => {
                                            creature.name = line_vec[1].clone();
                                            layer_set = LayerSet::Simple(Vec::new());
                                            dbg!("creature graphics");
                                            dbg!(&creature);
                                        },
                                        "STATUE_CREATURE_GRAPHICS" => {
                                            creature.name = line_vec[1].clone();
                                            creature.graphics_type.clear();
                                            layer_set = LayerSet::Statue(Vec::new());
                                        },
                                        "LAYER_SET" => {
                                            creature.graphics_type.clear();
                                            layer_set = LayerSet::Layered(State::from(line_vec[1].clone()),
                                            Vec::new());
                                            dbg!("layered");
                                            dbg!(&creature);
                                        },
                                        _ => {},
                                    }
                                },
                                "LAYER_GROUP" | "END_LAYER_GROUP" => {
                                    //handle explicit layer groups
                                    match &mut layer_set {
                                        LayerSet::Layered(_, layer_groups) => {
                                            if layer.name.ne("") {
                                                layer.conditions.push(condition.clone());
                                                layer_group.layers.push(layer.clone());
                                                layer = Layer::empty();
                                                layer_groups.push(layer_group);
                                                layer_group = LayerGroup::empty();
                                            }
                                        },
                                        _ => { panic!("should not see layer groups outside of a 'layered' layer set"); }
                                    }
                                },
                                "LAYER" => {
                                    match &mut layer_set {
                                        LayerSet::Layered(..) => {
                                            if layer.name.ne("") {
                                                layer.conditions.push(condition.clone());
                                                layer_group.layers.push(layer.clone());
                                            }
                                            if line_vec[3].eq("LARGE_IMAGE") {
                                                layer = Layer{
                                                    name: line_vec[1].clone(),
                                                    conditions: Vec::new(),
                                                    tile: line_vec[2].clone(),
                                                    coords:
                                                        [line_vec[4].parse::<usize>().unwrap_or_default(),
                                                        line_vec[5].parse::<usize>().unwrap_or_default()],
                                                    large_coords:
                                                        Some([line_vec[6].parse::<usize>().unwrap_or_default(),
                                                        line_vec[7].parse::<usize>().unwrap_or_default()]),
                                                }
                                            } else {
                                                layer = Layer{
                                                    name: line_vec[1].clone(),
                                                    conditions: Vec::new(),
                                                    tile: line_vec[2].clone(),
                                                    coords:
                                                        [line_vec[3].parse::<usize>().unwrap_or_default(),
                                                        line_vec[4].parse::<usize>().unwrap_or_default()],
                                                    large_coords: None,
                                                }
                                            }
                                        },
                                        _ => { panic!("should not see layers outside of a 'layered' layer set"); }
                                    }
                                },
                                _ => {//if there's a bracketed tag that is not already covered
                                    match &mut layer_set {
                                        LayerSet::Simple(simple_layers) => {
                                            if simple_layer.state.ne(&State::Empty) {
                                                simple_layers.push(simple_layer.clone());
                                            }
                                            if line_vec[2].eq("LARGE_IMAGE") {
                                                simple_layer = SimpleLayer {
                                                    state: State::from(line_vec[0].clone()),
                                                    tile: line_vec[1].clone(),
                                                    coords: 
                                                        [line_vec[3].parse::<usize>().unwrap_or_default(),
                                                        line_vec[4].parse::<usize>().unwrap_or_default()],
                                                    large_coords: 
                                                        Some([line_vec[5].parse::<usize>().unwrap_or_default(),
                                                        line_vec[6].parse::<usize>().unwrap_or_default()]),
                                                    sub_state: {
                                                        if line_vec.get(7).is_some() {
                                                            Some(State::from(line_vec[7].clone()))
                                                        } else {
                                                            None
                                                        }
                                                    }
                                                }
                                            } else {
                                                simple_layer = SimpleLayer {
                                                    state: State::from(line_vec[0].clone()),
                                                    tile: line_vec[1].clone(),
                                                    coords: 
                                                        [line_vec[2].parse::<usize>().unwrap_or_default(),
                                                        line_vec[3].parse::<usize>().unwrap_or_default()],
                                                    large_coords: None,
                                                    sub_state: {
                                                        if line_vec.get(4).is_some() {
                                                            Some(State::from(line_vec[4].clone()))
                                                        } else {
                                                            None
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                        LayerSet::Statue(simple_layers) => {
                                            if simple_layer.state.ne(&State::Empty) {
                                                simple_layers.push(simple_layer.clone());
                                            }
                                            simple_layer = SimpleLayer {
                                                state: State::from(line_vec[0].clone()),
                                                tile: line_vec[1].clone(),
                                                coords: 
                                                    [line_vec[2].parse::<usize>().unwrap_or_default(),
                                                    line_vec[3].parse::<usize>().unwrap_or_default()],
                                                large_coords: 
                                                    Some([line_vec[4].parse::<usize>().unwrap_or_default(),
                                                    line_vec[5].parse::<usize>().unwrap_or_default()]),
                                                sub_state: None
                                            }
                                        },
                                        LayerSet::Layered(..) => {
                                            if condition.ne(&Condition::default()) {
                                                layer.conditions.push(condition.clone());
                                            }
                                            condition = Condition::from(line_vec.clone());
                                        },
                                        _ => {}
                                    }
                                }                                
                            }
                        }
                    }

                    //push everything down at end of file
                    layer.conditions.push(condition);
                    layer_group.layers.push(layer);
                    dbg!(layer_set.clone());
                    match &mut layer_set {
                        LayerSet::Empty => {},
                        LayerSet::Layered(_, layer_groups) => {
                            layer_groups.push(layer_group);
                            dbg!(layer_set.clone());
                            creature.graphics_type.push(layer_set);
                            dbg!(&creature);
                            creature_file.creatures.push(creature);
                        },
                        LayerSet::Simple(simple_layers) | LayerSet::Statue(simple_layers) => {
                            simple_layers.push(simple_layer);
                            creature.graphics_type.push(layer_set);
                            creature_file.creatures.push(creature);
                        }
                    }
                    creature_files.push(creature_file);
                }
            }
        }

        Graphics {
            tilepages: tilepages,
            creature_files: creature_files,
            ..Default::default()
        }
    }

    fn export(&self) {
        todo!("export");
        // fs::DirBuilder::new()
        //     .recursive(true)
        //     .create("./graphics")
        //     .unwrap();
        // fs::DirBuilder::new()
        //     .recursive(true)
        //     .create("./graphics/images")
        //     .unwrap();

        // for tilepage in self.tilepages.iter() {
        //     //iterate over Tile Pages
        //     let tilepage_name = tilepage.name.as_str();
        //     let tilepage_file = fs::File::create(format!(
        //         "./graphics/tile_page_{}.txt",
        //         tilepage_name.to_case(Case::Snake)
        //     ))
        //     .expect("tile page file creation failed");
        //     let mut tilepage_file = io::LineWriter::new(tilepage_file);

        //     tilepage_file
        //         .write_all(
        //             //Heading
        //             format!(
        //                 "tile_page_{}\n\n[OBJECT:TILE_PAGE]\n\n",
        //                 tilepage_name.to_case(Case::Snake)
        //             )
        //             .as_bytes(),
        //         )
        //         .expect("why here failed");

        //     for tile in tilepage.tiles.iter() {
        //         //iterate through tiles
        //         tilepage_file
        //             .write_all(
        //                 format!("[TILE_PAGE:{}]\n", tile.name.to_case(Case::UpperSnake)).as_bytes(),
        //             )
        //             .expect("why here failed 2");
        //         tilepage_file
        //             .write_all(
        //                 format!(
        //                     "\t[FILE:image/{}.png]\n",
        //                     tile.filename.to_case(Case::Snake).as_str()
        //                 )
        //                 .as_bytes(),
        //             )
        //             .expect("why here failed 3");
        //         tilepage_file
        //             .write_all(
        //                 format!("\t[TILE_DIM:{}:{}]\n", tile.tile_size[0], tile.tile_size[1])
        //                     .as_bytes(),
        //             )
        //             .expect("why here failed 4");
        //         tilepage_file
        //             .write_all(
        //                 format!(
        //                     "\t[PAGE_DIM_PIXELS:{}:{}]\n\n",
        //                     tile.image_size[0], tile.image_size[1]
        //                 )
        //                 .as_bytes(),
        //             )
        //             .expect("why here failed 5");
        //     }
        //     tilepage_file
        //         .flush()
        //         .expect("tile page file failed to finalize."); //finalize file writing
        // }
    }
}

#[derive(Clone, Debug, Default)]
struct CreatureFile {
    name: String,             //graphcics_creatures_ name
    creatures: Vec<Creature>, //set of creatures/creature graphics types to group in one file
}
impl CreatureFile {
    fn new() -> CreatureFile {
        CreatureFile {
            name: String::new(),
            creatures: vec![Creature::new()],
        }
    }

    fn empty() -> CreatureFile {
        CreatureFile {
            name: String::new(),
            creatures: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
enum State {
    #[default]
    Empty,
    Default,
    Child, //todo add rest
    Custom(String),
}
impl State {
    fn name(&self) -> String {
        match self {
            State::Default => "DEFAULT".to_string(),
            State::Child => "CHILD".to_string(),//todo add rest
            State::Custom(name) => name.to_string(),
            _ => "(unexpected state)".to_string(),
        }
    }
}
impl From<String> for State {
    fn from(string: String) -> Self {
        match string.to_uppercase().as_str() {
            "DEFAULT" => {State::Default},
            "CHILD" => {State::Child},//todo add rest
            other => { State::Custom(other.to_string()) }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
enum LayerSet {
    #[default]
    Empty,
    Simple(Vec<SimpleLayer>),
    Statue(Vec<SimpleLayer>),
    Layered(State, Vec<LayerGroup>),
}
impl LayerSet {
    fn name(&self) -> String {
        match self {
            LayerSet::Simple(..) => "SIMPLE".to_string(),
            LayerSet::Layered(..) => "LAYERED".to_string(),
            LayerSet::Empty => "(none)".to_string(),
            _ => "(unexpected state)".to_string(),
        }
    }
}

#[derive(Clone, Default, Debug)]
struct Creature {
    name: String,                 //file name of creature_graphics_file_name.txt
    graphics_type: Vec<LayerSet>, //which type of graphics (layered, simple, caste, fb)
}
impl Creature {
    fn new() -> Creature {
        Creature {
            name: String::from("(new)"),
            graphics_type: vec![LayerSet::default()],
        }
    }

    fn empty() -> Creature {
        Creature {
            name: String::new(),
            graphics_type: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct SimpleLayer {
    state: State,
    tile: String,
    coords: [usize; 2],
    large_coords: Option<[usize; 2]>,
    sub_state: Option<State>,
}
impl SimpleLayer {
    fn empty() -> SimpleLayer {
        SimpleLayer {
            state: State::default(),
            tile: String::new(),
            coords: [0, 0],
            large_coords: None,
            sub_state: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Layer {
    name: String,                     //LAYER_NAME for patterning
    conditions: Vec<Condition>,       //Set of condition(s) that layer displays in
    tile: String,                     //TILE_NAME of image
    coords: [usize; 2],               //x,y coordinates of layer on image in tiles
    large_coords: Option<[usize; 2]>, //(optional) x2,y2 coordinates of bottom right corner of layer in tiles
}
impl Layer {
    fn new() -> Layer {
        Layer {
            name: "new".to_string(),
            conditions: vec![Condition::default()],
            tile: String::new(),
            coords: [0, 0],
            large_coords: None,
        }
    }

    fn empty() -> Layer {
        Layer {
            name: String::new(),
            conditions: Vec::new(),
            tile: String::new(),
            coords: [0, 0],
            large_coords: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct LayerGroup {
    name: String,       //internal layer group name
    layers: Vec<Layer>, //set of layers to display for creature
}
impl LayerGroup {
    fn new() -> LayerGroup {
        LayerGroup {
            name: "new".to_string(),
            layers: vec![Layer::new()],
        }
    }

    fn empty() -> LayerGroup {
        LayerGroup {
            name: String::new(),
            layers: Vec::new(),
        }
    }
}

// enum MaterialType {}
// enum Material {}
// enum ItemType {}

#[derive(Clone, Debug, Default, PartialEq)]
enum Condition {
    #[default]
    Default,
    ItemWorn,
    ShutOffIfItemPresent,
    Dye(String),
    NotDyed,
    MaterialFlag,
    MaterialType,
    ProfessionCategory(Vec<String>),
    RandomPartIndex(String, usize, usize),
    HaulCountMin(usize),
    HaulCountMax(usize),
    Child,
    NotChild,
    Caste(String),
    Ghost,
    SynClass(String),
    TissueLayer,
    TissueMinLength(usize),
    TissueMaxLength(usize),
    TissueMayHaveColor(Vec<String>),
    TissueMayHaveShaping(Vec<String>),
    TissueNotShaped,
    TissueSwap(String, usize, String, [usize;2], Option<[usize;2]>),
    Custom(String),
}
impl Condition {
    fn name(&self) -> String {
        match self {
            Condition::Default => "(default)".to_string(),
            Condition::ItemWorn => "CONDITION_ITEM_WORN".to_string(),
            Condition::ShutOffIfItemPresent => "SHUT_OFF_IF_ITEM_PRESENT".to_string(),
            Condition::Dye(..) => "CONDITION_DYE".to_string(),
            Condition::NotDyed => "CONDITION_NOT_DYED".to_string(),
            Condition::MaterialFlag => "CONDITION_MATERIAL_FLAG".to_string(),
            Condition::MaterialType => "CONDITION_MATERIAL_TYPE".to_string(),
            Condition::ProfessionCategory(..) => "CONDITION_PROFESSION_CATEGORY".to_string(),
            Condition::RandomPartIndex(..) => "CONDITION_RANDOM_PART_INDEX".to_string(),
            Condition::HaulCountMin(..) => "CONDITION_HAUL_COUNT_MIN".to_string(),
            Condition::HaulCountMax(..) => "CONDITION_HAUL_COUNT_MAX".to_string(),
            Condition::Child => "CONDITION_CHILD".to_string(),
            Condition::NotChild => "CONDITION_NOT_CHILD".to_string(),
            Condition::Caste(..) => "CONDITION_CASTE".to_string(),
            Condition::Ghost => "CONDITION_GHOST".to_string(),
            Condition::SynClass(..) => "CONDITION_SYN_CLASS".to_string(),
            Condition::TissueLayer => "CONDITION_TISSUE_LAYER".to_string(),
            Condition::TissueMinLength(..) => "TISSUE_MIN_LENGTH".to_string(),
            Condition::TissueMaxLength(..) => "TISSUE_MAX_LENGTH".to_string(),
            Condition::TissueMayHaveColor(..) => "TISSUE_MAY_HAVE_COLOR".to_string(),
            Condition::TissueMayHaveShaping(..) => "TISSUE_MAY_HAVE_SHAPING".to_string(),
            Condition::TissueNotShaped => "TISSUE_NOT_SHAPED".to_string(),
            Condition::TissueSwap(..) => "TISSUE_SWAP".to_string(),
            Condition::Custom(string) => string.clone(),
        }
    }

    fn from(mut line_vec: Vec<String>) -> Condition {
        match line_vec[0].as_str() {
            "(default)" => Condition::Default,
            "CONDITION_ITEM_WORN" => Condition::ItemWorn,
            "SHUT_OFF_IF_ITEM_PRESENT" => Condition::ShutOffIfItemPresent,
            "CONDITION_DYE" => Condition::Dye(
                line_vec[1].clone()
            ),
            "CONDITION_NOT_DYED" => Condition::NotDyed,
            "CONDITION_MATERIAL_FLAG" => Condition::MaterialFlag,
            "CONDITION_MATERIAL_TYPE" => Condition::MaterialType,
            "CONDITION_PROFESSION_CATEGORY" => Condition::ProfessionCategory(
                line_vec.drain(1..).collect()
            ),
            "CONDITION_RANDOM_PART_INDEX" => Condition::RandomPartIndex(
                line_vec[1].clone(),
                line_vec[2].parse::<usize>().unwrap_or_default(),
                line_vec[3].parse::<usize>().unwrap_or_default()
            ),
            "CONDITION_HAUL_COUNT_MIN" => Condition::HaulCountMin(
                line_vec[1].parse::<usize>().unwrap_or_default()
            ),
            "CONDITION_HAUL_COUNT_MAX" => Condition::HaulCountMax(
                line_vec[1].parse::<usize>().unwrap_or_default()
            ),
            "CONDITION_CHILD" => Condition::Child,
            "CONDITION_NOT_CHILD" => Condition::NotChild,
            "CONDITION_CASTE" => Condition::Caste(
                line_vec[1].clone()
            ),
            "CONDITION_GHOST" => Condition::Ghost,
            "CONDITION_SYN_CLASS" => Condition::SynClass(
                line_vec[1].clone()
            ),
            "CONDITION_TISSUE_LAYER" => Condition::TissueLayer,
            "TISSUE_MIN_LENGTH" => Condition::TissueMinLength(
                line_vec[1].parse::<usize>().unwrap_or_default()
            ),
            "TISSUE_MAX_LENGTH" => Condition::TissueMaxLength(
                line_vec[1].parse::<usize>().unwrap_or_default()
            ),
            "TISSUE_MAY_HAVE_COLOR" => Condition::TissueMayHaveColor(
                line_vec.drain(1..).collect()
            ),
            "TISSUE_MAY_HAVE_SHAPING" => Condition::TissueMayHaveShaping(
                line_vec.drain(1..).collect()
            ),
            "TISSUE_NOT_SHAPED" => Condition::TissueNotShaped,
            "TISSUE_SWAP" => {
                if line_vec[4].eq("LARGE_IMAGE") {
                    Condition::TissueSwap(
                        line_vec[1].clone(),
                        line_vec[2].parse::<usize>().unwrap_or_default(),
                        line_vec[3].clone(),
                        [line_vec[5].parse::<usize>().unwrap_or_default(), 
                        line_vec[6].parse::<usize>().unwrap_or_default()],
                        Some(
                            [line_vec[7].parse::<usize>().unwrap_or_default(), 
                            line_vec[8].parse::<usize>().unwrap_or_default()]
                        ),
                    )
                } else {
                    Condition::TissueSwap(
                        line_vec[1].clone(),
                        line_vec[2].parse::<usize>().unwrap_or_default(),
                        line_vec[3].clone(),
                        [line_vec[4].parse::<usize>().unwrap_or_default(), 
                        line_vec[5].parse::<usize>().unwrap_or_default()],
                        None,
                    )
                }
            },
            other => Condition::Custom(other.to_string()),
        }
    }

    fn condition_menu(&mut self, ui: &mut Ui, tile_names: &Vec<String>) {
        egui::ComboBox::from_label("Condition type")
            .selected_text(&self.name())
            .show_ui(ui, |ui| {
                ui.selectable_value(self, Condition::Default, "(select)");
                ui.selectable_value(self, Condition::ItemWorn, "CONDITION_ITEM_WORN");
                ui.selectable_value(
                    self,
                    Condition::ShutOffIfItemPresent,
                    "SHUT_OFF_IF_ITEM_PRESENT",
                );
                ui.selectable_value(self, Condition::Dye(String::new()), "CONDITION_DYE");
                ui.selectable_value(self, Condition::NotDyed, "CONDITION_NOT_DYED");
                ui.selectable_value(self, Condition::MaterialFlag, "CONDITION_MATERIAL_FLAG");
                ui.selectable_value(self, Condition::MaterialType, "CONDITION_MATERIAL_TYPE");
                ui.selectable_value(
                    self,
                    Condition::ProfessionCategory(Vec::new()),
                    "CONDITION_PROFESSION_CATEGORY",
                );
                ui.selectable_value(
                    self,
                    Condition::RandomPartIndex(String::new(), 1, 1),
                    "CONDITION_RANDOM_PART_INDEX",
                );
                ui.selectable_value(self, Condition::HaulCountMin(0), "CONDITION_HAUL_COUNT_MIN");
                ui.selectable_value(self, Condition::HaulCountMax(0), "CONDITION_HAUL_COUNT_MAX");
                ui.selectable_value(self, Condition::Child, "CONDITION_CHILD");
                ui.selectable_value(self, Condition::NotChild, "CONDITION_NOT_CHILD");
                ui.selectable_value(
                    self,
                    Condition::Caste(String::from("MALE")),
                    "CONDITION_CASTE",
                );
                ui.selectable_value(self, Condition::Ghost, "CONDITION_GHOST");
                ui.selectable_value(
                    self,
                    Condition::SynClass(String::new()),
                    "CONDITION_SYN_CLASS",
                );
                ui.selectable_value(self, Condition::TissueLayer, "CONDITION_TISSUE_LAYER");
                ui.selectable_value(self, Condition::TissueMinLength(0), "TISSUE_MIN_LENGTH");
                ui.selectable_value(self, Condition::TissueMaxLength(0), "TISSUE_MAX_LENGTH");
                ui.selectable_value(
                    self,
                    Condition::TissueMayHaveColor(vec![String::new()]),
                    "TISSUE_MAY_HAVE_COLOR",
                );
                ui.selectable_value(
                    self,
                    Condition::TissueMayHaveShaping(vec![String::new()]),
                    "TISSUE_MAY_HAVE_SHAPING",
                );
                ui.selectable_value(self, Condition::TissueNotShaped, "TISSUE_NOT_SHAPED");
                ui.selectable_value(
                    self,
                    Condition::TissueSwap(String::from("IF_MIN_CURLY"), 0, String::new(), [0,0], None),
                    "TISSUE_SWAP",
                );
            });

        ui.add_space(PADDING);

        match self {
            Condition::ItemWorn => {
                //todo
                todo!();
                // egui::ComboBox::from_label("Selection type")
                //     .selected_text(self.contents.get(0).unwrap())
                //     .show_ui(ui, |ui| {
                //         ui.selectable_value(
                //             self.contents.get_mut(0).unwrap(),
                //             String::from(""),
                //             "(select)",
                //         );
                //         ui.selectable_value(
                //             self.contents.get_mut(0).unwrap(),
                //             String::from("BY_CATEGORY"),
                //             "BY_CATEGORY",
                //         );
                //         ui.selectable_value(
                //             self.contents.get_mut(0).unwrap(),
                //             String::from("BY_TOKEN"),
                //             "BY_TOKEN",
                //         );
                //         ui.selectable_value(
                //             self.contents.get_mut(0).unwrap(),
                //             String::from("ANY_HELD"),
                //             "ANY_HELD",
                //         );
                //         ui.selectable_value(
                //             self.contents.get_mut(0).unwrap(),
                //             String::from("WIELD"),
                //             "WIELD",
                //         );
                //     });
                // ui.label("Selection subtype:");
                // match self.contents.get(0).unwrap().as_str() {
                //     "BY_CATEGORY" => {
                //         if self.contents.len() < 4 {
                //             self.contents.push("".to_string());
                //         } else {
                //             ui.label("Category: (e.g. HEAD)");
                //             ui.text_edit_singleline(self.contents.get_mut(1).unwrap());
                //             ui.label("Item type: (e.g. HELM)");
                //             ui.text_edit_singleline(self.contents.get_mut(2).unwrap());
                //             ui.label("Item: (e.g. ITEM_HELM_HELM)");
                //             ui.text_edit_singleline(self.contents.get_mut(3).unwrap());
                //             if self.contents.len() > 4 {
                //                 for i in 4..self.contents.len() {
                //                     ui.label("Item:");
                //                     ui.text_edit_singleline(self.contents.get_mut(i).unwrap());
                //                 }
                //             }
                //             ui.horizontal(|ui| {
                //                 if ui.button("Add item").clicked() {
                //                     self.contents.push("".into());
                //                 }
                //                 if ui.button("Remove item").clicked() && self.contents.len() > 4
                //                 {
                //                     self.contents.pop();
                //                 }
                //             });
                //         }
                //         ui.add_space(PADDING);
                //     }
                //     "BY_TOKEN" => {
                //         if self.contents.len() < 4 {
                //             self.contents.push("".to_string());
                //         } else {
                //             ui.label("Token: (e.g. RH for right hand)");
                //             ui.text_edit_singleline(self.contents.get_mut(1).unwrap());
                //             ui.label("Item type: (e.g. GLOVES)");
                //             ui.text_edit_singleline(self.contents.get_mut(2).unwrap());
                //             ui.label("Item: (e.g. ITEM_GLOVES_GAUNTLETS)");
                //             ui.text_edit_singleline(self.contents.get_mut(3).unwrap());
                //             if self.contents.len() > 4 {
                //                 for i in 4..self.contents.len() {
                //                     ui.label("Item:");
                //                     ui.text_edit_singleline(self.contents.get_mut(i).unwrap());
                //                 }
                //             }
                //             ui.horizontal(|ui| {
                //                 if ui.button("Add item").clicked() {
                //                     self.contents.push("".into());
                //                 }
                //                 if ui.button("Remove item").clicked() && self.contents.len() > 4
                //                 {
                //                     self.contents.pop();
                //                 }
                //             });
                //         }
                //         ui.add_space(PADDING);
                //     }
                //     "ANY_HELD" => {
                //         if self.contents.len() > 3 {
                //             self.contents.pop();
                //         } else if self.contents.len() < 3 {
                //             self.contents.push("".to_string());
                //         } else {
                //             ui.label("Held type: (e.g. SHIELD)");
                //             ui.text_edit_singleline(self.contents.get_mut(1).unwrap());
                //             ui.label("Held item: (e.g. ITEM_SHIELD_SHIELD)");
                //             ui.text_edit_singleline(self.contents.get_mut(2).unwrap());
                //         }
                //         ui.add_space(PADDING);
                //     }
                //     "WIELD" => {
                //         if self.contents.len() > 3 {
                //             self.contents.pop();
                //         } else {
                //             ui.label("Wielded type: (e.g. WEAPON or ANY)");
                //             ui.text_edit_singleline(self.contents.get_mut(1).unwrap());
                //             if self.contents.get(1).unwrap().ne("ANY") {
                //                 if self.contents.len() < 3 {
                //                     self.contents.push("".to_string());
                //                 }
                //                 ui.label("Wielded item: (e.g. ITEM_WEAPON_PICK)");
                //                 ui.text_edit_singleline(self.contents.get_mut(2).unwrap());
                //             } else {
                //                 if self.contents.len() > 2 {
                //                     self.contents.pop();
                //                 }
                //             }
                //         }
                //         ui.add_space(PADDING);
                //     }
                //     _ => {
                //         ui.add_space(PADDING);
                //     }
                // }
            }
            Condition::ShutOffIfItemPresent => {
                //todo
                todo!()
                // egui::ComboBox::from_label("Selection type")
                //     .selected_text(self.contents.get(0).unwrap())
                //     .show_ui(ui, |ui| {
                //         ui.selectable_value(
                //             self.contents.get_mut(0).unwrap(),
                //             String::from(""),
                //             "(select)",
                //         );
                //         ui.selectable_value(
                //             self.contents.get_mut(0).unwrap(),
                //             String::from("BY_CATEGORY"),
                //             "BY_CATEGORY",
                //         );
                //         ui.selectable_value(
                //             self.contents.get_mut(0).unwrap(),
                //             String::from("BY_TOKEN"),
                //             "BY_TOKEN",
                //         );
                //         ui.selectable_value(
                //             self.contents.get_mut(0).unwrap(),
                //             String::from("ANY_HELD"),
                //             "ANY_HELD",
                //         );
                //         ui.selectable_value(
                //             self.contents.get_mut(0).unwrap(),
                //             String::from("WIELD"),
                //             "WIELD",
                //         );
                //     });
                // ui.label("Selection subtype:");
                // match self.contents.get(0).unwrap().as_str() {
                //     "BY_CATEGORY" => {
                //         if self.contents.len() < 4 {
                //             self.contents.push("".to_string());
                //         } else {
                //             ui.label("Category: (e.g. HEAD)");
                //             ui.text_edit_singleline(self.contents.get_mut(1).unwrap());
                //             ui.label("Item type: (e.g. HELM)");
                //             ui.text_edit_singleline(self.contents.get_mut(2).unwrap());
                //             ui.label("Item: (e.g. ITEM_HELM_HELM)");
                //             ui.text_edit_singleline(self.contents.get_mut(3).unwrap());
                //             if self.contents.len() > 4 {
                //                 for i in 4..self.contents.len() {
                //                     ui.label("Item:");
                //                     ui.text_edit_singleline(self.contents.get_mut(i).unwrap());
                //                 }
                //             }
                //             ui.horizontal(|ui| {
                //                 if ui.button("Add item").clicked() {
                //                     self.contents.push("".into());
                //                 }
                //                 if ui.button("Remove item").clicked() && self.contents.len() > 4
                //                 {
                //                     self.contents.pop();
                //                 }
                //             });
                //         }
                //         ui.add_space(PADDING);
                //     }
                //     "BY_TOKEN" => {
                //         if self.contents.len() < 4 {
                //             self.contents.push("".to_string());
                //         } else {
                //             ui.label("Token: (e.g. RH for right hand)");
                //             ui.text_edit_singleline(self.contents.get_mut(1).unwrap());
                //             ui.label("Item type: (e.g. GLOVES)");
                //             ui.text_edit_singleline(self.contents.get_mut(2).unwrap());
                //             ui.label("Item: (e.g. ITEM_GLOVES_GAUNTLETS)");
                //             ui.text_edit_singleline(self.contents.get_mut(3).unwrap());
                //             if self.contents.len() > 4 {
                //                 for i in 4..self.contents.len() {
                //                     ui.label("Item:");
                //                     ui.text_edit_singleline(self.contents.get_mut(i).unwrap());
                //                 }
                //             }
                //             ui.horizontal(|ui| {
                //                 if ui.button("Add item").clicked() {
                //                     self.contents.push("".into());
                //                 }
                //                 if ui.button("Remove item").clicked() && self.contents.len() > 4
                //                 {
                //                     self.contents.pop();
                //                 }
                //             });
                //         }
                //         ui.add_space(PADDING);
                //     }
                //     "ANY_HELD" => {
                //         if self.contents.len() > 3 {
                //             self.contents.pop();
                //         } else if self.contents.len() < 3 {
                //             self.contents.push("".to_string());
                //         } else {
                //             ui.label("Held type: (e.g. SHIELD)");
                //             ui.text_edit_singleline(self.contents.get_mut(1).unwrap());
                //             ui.label("Held item: (e.g. ITEM_SHIELD_SHIELD)");
                //             ui.text_edit_singleline(self.contents.get_mut(2).unwrap());
                //         }
                //         ui.add_space(PADDING);
                //     }
                //     "WIELD" => {
                //         if self.contents.len() > 3 {
                //             self.contents.pop();
                //         } else {
                //             ui.label("Wielded type: (e.g. WEAPON or ANY)");
                //             ui.text_edit_singleline(self.contents.get_mut(1).unwrap());
                //             if self.contents.get(1).unwrap().ne("ANY") {
                //                 if self.contents.len() < 3 {
                //                     self.contents.push("".to_string());
                //                 }
                //                 ui.label("Wielded item: (e.g. ITEM_WEAPON_PICK)");
                //                 ui.text_edit_singleline(self.contents.get_mut(2).unwrap());
                //             } else {
                //                 if self.contents.len() > 2 {
                //                     self.contents.pop();
                //                 }
                //             }
                //         }
                //         ui.add_space(PADDING);
                //     }
                //     _ => {
                //         ui.add_space(PADDING);
                //     }
                // }
            }
            Condition::Dye(dye) => {
                ui.hyperlink_to(
                    "Dye color token:",
                    "http://dwarffortresswiki.org/index.php/Color#Color_tokens",
                );
                ui.text_edit_singleline(dye);
                ui.label("Not functional in v50.05");
            }
            Condition::NotDyed => {
                ui.label("No additional input needed.\n\nNot functional currently (v50.05)");
            }
            Condition::MaterialFlag => {
                //todo
                todo!();
                // for flag in self.contents.iter_mut() {
                //     ui.push_id(flag.to_string(), |ui| {
                //         egui::ComboBox::from_label(
                //             "Material flag:   (dropdown contains common ones)",
                //         )
                //         .selected_text(flag.to_string())
                //         .show_ui(ui, |ui| {
                //             ui.selectable_value(flag, String::from(""), "(select)");
                //             ui.selectable_value(
                //                 flag,
                //                 String::from("NOT_ARTIFACT"),
                //                 "NOT_ARTIFACT",
                //             );
                //             ui.selectable_value(
                //                 flag,
                //                 String::from("IS_CRAFTED_ARTIFACT"),
                //                 "IS_CRAFTED_ARTIFACT",
                //             );
                //             ui.selectable_value(
                //                 flag,
                //                 String::from("IS_DIVINE_MATERIAL"),
                //                 "IS_DIVINE_MATERIAL",
                //             );
                //             ui.selectable_value(flag, String::from("WOVEN_ITEM"), "WOVEN_ITEM");
                //             ui.selectable_value(
                //                 flag,
                //                 String::from("ANY_WOOD_MATERIAL"),
                //                 "ANY_WOOD_MATERIAL",
                //             );
                //             ui.selectable_value(
                //                 flag,
                //                 String::from("ANY_LEATHER_MATERIAL"),
                //                 "ANY_LEATHER_MATERIAL",
                //             );
                //             ui.selectable_value(
                //                 flag,
                //                 String::from("ANY_BONE_MATERIAL"),
                //                 "ANY_BONE_MATERIAL",
                //             );
                //             ui.selectable_value(
                //                 flag,
                //                 String::from("ANY_SHELL_MATERIAL"),
                //                 "ANY_SHELL_MATERIAL",
                //             );
                //             ui.selectable_value(
                //                 flag,
                //                 String::from("METAL_ITEM_MATERIAL"),
                //                 "METAL_ITEM_MATERIAL",
                //             );
                //         });
                //         ui.text_edit_singleline(flag);
                //     });
                // }
                // ui.horizontal(|ui| {
                //     if ui.button("Add flag").clicked() {
                //         self.contents.push("".into());
                //     }
                //     if ui.button("Remove flag").clicked() && self.contents.len() > 1 {
                //         self.contents.pop();
                //     }
                // });

                // ui.add_space(PADDING);
                // ui.hyperlink_to("List of more useful flags.", "https://dwarffortresswiki.org/index.php/Graphics_token#CONDITION_MATERIAL_FLAG");
                // ui.hyperlink_to("Full list of all possible flags (v50.05).", "http://www.bay12forums.com/smf/index.php?topic=169696.msg8442543#msg8442543");
            }
            Condition::MaterialType => {
                //todo
                todo!();
                // ui.label("Material token : (\"METAL\" is the only one known to work v50.05)");
                // ui.text_edit_singleline(self.contents.get_mut(0).unwrap());
                // egui::ComboBox::from_label(
                //     "Material name:   (dropdown contains vanilla weapon metals)",
                // )
                // .selected_text(self.contents.get(1).unwrap())
                // .show_ui(ui, |ui| {
                //     ui.selectable_value(
                //         self.contents.get_mut(1).unwrap(),
                //         String::from(""),
                //         "(select)",
                //     );
                //     ui.selectable_value(
                //         self.contents.get_mut(1).unwrap(),
                //         String::from("COPPER"),
                //         "COPPER",
                //     );
                //     ui.selectable_value(
                //         self.contents.get_mut(1).unwrap(),
                //         String::from("SILVER"),
                //         "SILVER",
                //     );
                //     ui.selectable_value(
                //         self.contents.get_mut(1).unwrap(),
                //         String::from("BRONZE"),
                //         "BRONZE",
                //     );
                //     ui.selectable_value(
                //         self.contents.get_mut(1).unwrap(),
                //         String::from("BLACK_BRONZE"),
                //         "BLACK_BRONZE",
                //     );
                //     ui.selectable_value(
                //         self.contents.get_mut(1).unwrap(),
                //         String::from("IRON"),
                //         "IRON",
                //     );
                //     ui.selectable_value(
                //         self.contents.get_mut(1).unwrap(),
                //         String::from("STEEL"),
                //         "STEEL",
                //     );
                //     ui.selectable_value(
                //         self.contents.get_mut(1).unwrap(),
                //         String::from("ADAMANTINE"),
                //         "ADAMANTINE",
                //     );
                // });
                // ui.text_edit_singleline(self.contents.get_mut(1).unwrap());

                // ui.add_space(PADDING);
                // ui.label(
                //     "In vanilla, only used for metal weapons and armor (e.g. METAL:IRON).",
                // );
                // ui.hyperlink_to("At least some material flags are currently unusable (v50.05 //todo recheck).", "https://dwarffortresswiki.org/index.php/Graphics_token#CONDITION_MATERIAL_TYPE");
            }
            Condition::ProfessionCategory(contents) => {
                for profession in contents.iter_mut() {
                    ui.push_id(profession.to_string(), |ui| {
                        egui::ComboBox::from_label("Profession:   (dropdown contains common ones)")
                            .selected_text(profession.to_string())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(profession, String::from(""), "(select)");
                                ui.selectable_value(profession, String::from("NONE"), "NONE");
                                ui.selectable_value(
                                    profession,
                                    String::from("STONEWORKER"),
                                    "STONEWORKER",
                                );
                                ui.selectable_value(profession, String::from("MINER"), "MINER");
                                ui.selectable_value(
                                    profession,
                                    String::from("MERCHANT"),
                                    "MERCHANT",
                                );
                                ui.selectable_value(
                                    profession,
                                    String::from("METALSMITH"),
                                    "METALSMITH",
                                );
                                ui.selectable_value(
                                    profession,
                                    String::from("ENGINEER"),
                                    "ENGINEER",
                                );
                                ui.selectable_value(profession, String::from("CHILD"), "CHILD");
                                ui.selectable_value(profession, String::from("FARMER"), "FARMER");
                                ui.selectable_value(
                                    profession,
                                    String::from("WOODWORKER"),
                                    "WOODWORKER",
                                );
                                ui.selectable_value(profession, String::from("JEWELER"), "JEWELER");
                                ui.selectable_value(profession, String::from("RANGER"), "RANGER");
                                ui.selectable_value(
                                    profession,
                                    String::from("STANDARD"),
                                    "STANDARD",
                                );
                                ui.selectable_value(
                                    profession,
                                    String::from("CRAFTSMAN"),
                                    "CRAFTSMAN",
                                );
                                ui.selectable_value(
                                    profession,
                                    String::from("FISHERY_WORKER"),
                                    "FISHERY_WORKER",
                                );
                            });
                        ui.text_edit_singleline(profession);
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("Add profession").clicked() {
                        contents.push("".into());
                    }
                    if ui.button("Remove profession").clicked() && contents.len() > 1 {
                        contents.pop();
                    }
                });

                ui.add_space(PADDING);
                ui.hyperlink_to(
                    "Full list of possible professions.",
                    "https://dwarffortresswiki.org/index.php/Unit_type_token",
                );
            }
            Condition::RandomPartIndex(id, index, max) => {
                ui.label("Random part identifier: (e.g. HEAD):");
                ui.text_edit_singleline(id);

                ui.add(
                    egui::DragValue::new(index)
                        .speed(1)
                        .prefix("Part index: ")
                        .clamp_range(0..=*max),
                );

                ui.add(
                    egui::DragValue::new(max)
                        .speed(1)
                        .prefix("Total parts: ")
                        .clamp_range(0..=usize::MAX),
                );

                ui.add_space(PADDING);
                ui.label(
                    "Allows multiple options for layers to be displayed for the same conditions.",
                );
                ui.label("To work, requires a set of layers with otherwise the same conditions and the same random part identifier.");
                ui.label("The part index and total parts are which random layer within the set this is (e.g. this is the 2nd (index) possible layer from 7 (total) options).");
            }
            Condition::HaulCountMin(haul_count) => {
                ui.add(
                    egui::DragValue::new(haul_count)
                        .speed(1)
                        .prefix("Min hauled items: "),
                );

                ui.add_space(PADDING);
                ui.label("Adds the layer based on how many items the creature is hauling (e.g. saddlebags)");
            }
            Condition::HaulCountMax(haul_count) => {
                ui.add(
                    egui::DragValue::new(haul_count)
                        .speed(1)
                        .prefix("Max hauled items: "),
                );

                ui.add_space(PADDING);
                ui.label("Adds the layer based on how many items the creature is hauling (e.g. saddlebags)");
            }
            Condition::Child => {
                ui.label("No additional input needed.");
            }
            Condition::NotChild => {
                ui.label("No additional input needed.");
            }
            Condition::Caste(caste) => {
                egui::ComboBox::from_label("Caste token: ")
                    .selected_text(caste.clone())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(caste, String::from("MALE"), "MALE");
                        ui.selectable_value(caste, String::from("FEMALE"), "FEMALE");
                        ui.selectable_value(caste, String::from("(custom)"), "(custom)");
                    });

                if "MALE".ne(caste) && "FEMALE".ne(caste) {
                    ui.text_edit_singleline(caste);
                }

                ui.add_space(PADDING);
                ui.label("Multiple caste conditions can be added.");
            }
            Condition::Ghost => {
                ui.label("No additional input needed.");
            }
            Condition::SynClass(syn_class) => {
                ui.hyperlink_to(
                    "Syndrome class:",
                    "https://dwarffortresswiki.org/index.php/Graphics_token#CONDITION_SYN_CLASS",
                );
                ui.text_edit_singleline(syn_class);
            }
            Condition::TissueLayer => {
                //todo
                todo!()
                // if self.contents.len() < 2 {
                //     self.contents.push("".to_string());
                // } else {
                //     egui::ComboBox::from_label("Selection type")
                //         .selected_text(self.contents.get(0).unwrap())
                //         .show_ui(ui, |ui| {
                //             ui.selectable_value(
                //                 self.contents.get_mut(0).unwrap(),
                //                 String::from(""),
                //                 "(select)",
                //             );
                //             ui.selectable_value(
                //                 self.contents.get_mut(0).unwrap(),
                //                 String::from("BY_CATEGORY"),
                //                 "BY_CATEGORY",
                //             );
                //             ui.selectable_value(
                //                 self.contents.get_mut(0).unwrap(),
                //                 String::from("BY_TOKEN"),
                //                 "BY_TOKEN",
                //             );
                //             ui.selectable_value(
                //                 self.contents.get_mut(0).unwrap(),
                //                 String::from("BY_TYPE"),
                //                 "BY_TYPE",
                //             );
                //         });
                //     ui.label("Selection subtype:");
                //     match self.contents.get(0).unwrap().as_str() {
                //         "BY_CATEGORY" => {
                //             if self.contents.len() < 3 {
                //                 self.contents.push("".to_string());
                //             } else {
                //                 ui.label("Category: (e.g. HEAD or ALL)");
                //                 ui.text_edit_singleline(self.contents.get_mut(1).unwrap());
                //                 ui.label("Tissue: (e.g. HAIR)");
                //                 ui.text_edit_singleline(self.contents.get_mut(2).unwrap());
                //             }
                //             ui.add_space(PADDING);
                //         }
                //         "BY_TOKEN" => {
                //             if self.contents.len() < 3 {
                //                 self.contents.push("".to_string());
                //             } else {
                //                 ui.label("Token: (e.g. RH for right hand)");
                //                 ui.text_edit_singleline(self.contents.get_mut(1).unwrap());
                //                 ui.label("Tissue: (e.g. SKIN)");
                //                 ui.text_edit_singleline(self.contents.get_mut(2).unwrap());
                //             }
                //             ui.add_space(PADDING);
                //         }
                //         "BY_TYPE" => {
                //             if self.contents.len() < 3 {
                //                 self.contents.push("".to_string());
                //             } else {
                //                 ui.label("Type: (e.g. GRASP)");
                //                 ui.text_edit_singleline(self.contents.get_mut(1).unwrap());
                //                 ui.label("Tissue: (e.g. SKIN)");
                //                 ui.text_edit_singleline(self.contents.get_mut(2).unwrap());
                //             }
                //             ui.add_space(PADDING);
                //         }
                //         _ => {
                //             ui.add_space(PADDING);
                //         }
                //     }
                // }
            }
            Condition::TissueMinLength(length) => {
                ui.add(egui::DragValue::new(length).speed(1).prefix("Min Length: "));

                ui.add_space(PADDING);
                ui.label("requires a CONDITION_TISSUE_LAYER above.");
            }
            Condition::TissueMaxLength(length) => {
                ui.add(egui::DragValue::new(length).speed(1).prefix("Max Length: "));

                ui.add_space(PADDING);
                ui.label("requires a CONDITION_TISSUE_LAYER above.");
            }
            Condition::TissueMayHaveColor(colors) => {
                ui.hyperlink_to(
                    "Color: (e.g. GRAY, RUST, MAROON)",
                    "https://dwarffortresswiki.org/index.php/Color#Color_tokens",
                );
                for color in colors {
                    ui.text_edit_singleline(color);
                }

                ui.add_space(PADDING);
                ui.label("requires a CONDITION_TISSUE_LAYER above.");
            }
            Condition::TissueMayHaveShaping(shapings) => {
                ui.hyperlink_to(
                    "Shaping: (e.g. NEATLY_COMBED, PONY_TAILS, CLEAN_SHAVEN)",
                    "https://dwarffortresswiki.org/index.php/Entity_token#TS_PREFERRED_SHAPING",
                );
                for shaping in shapings {
                    ui.text_edit_singleline(shaping);
                }

                ui.label("Additional shapings are used within graphics_creatures_creatures_layered.txt, but the complete list is not readily prepared.");

                ui.add_space(PADDING);
                ui.label("requires a CONDITION_TISSUE_LAYER above.");
            }
            Condition::TissueNotShaped => {
                ui.add_space(PADDING);
                ui.label("requires a CONDITION_TISSUE_LAYER above.");
                ui.label("No additional input needed.");
            }
            Condition::TissueSwap(app_mod, amount, tile, [x1,y1], mut large_coords) => {
                egui::ComboBox::from_label(
                    "Appearance Modifier (only IF_MIN_CURLY supported (v50.05)):",
                )
                .selected_text(app_mod.clone())
                .show_ui(ui, |ui| {
                    ui.selectable_value(app_mod, String::from("IF_MIN_CURLY"), "(select)");
                    ui.selectable_value(app_mod, String::from("IF_MIN_CURLY"), "IF_MIN_CURLY");
                });

                ui.add(
                    egui::DragValue::new(amount)
                        .speed(1)
                        .prefix("Modifier threshold: "),
                );

                egui::ComboBox::from_label("Tile for swapped layer: ")
                    .selected_text(tile.clone())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(tile, String::from(""), "(select)");
                        for tile_name in tile_names {
                            ui.selectable_value(tile, tile_name.to_string(), tile_name);
                        }
                    });

                ui.add(egui::DragValue::new(x1).speed(1).prefix("Tile X: "));
                ui.add(egui::DragValue::new(y1).speed(1).prefix("Tile Y: "));

                let mut large = large_coords.is_some();
                ui.checkbox(&mut large, "Large Image?:");

                if large {
                    if large_coords.is_some() {
                        let [mut x2,mut y2] = large_coords.unwrap();

                        ui.add(egui::Slider::new(&mut x2, *x1..=(*x1+2)));
                        ui.add(egui::Slider::new(&mut y2, *y1..=(*y1+1)));

                        large_coords = Some([x2,y2]);
                    } else {
                        large_coords = Some([*x1,*y1]);
                    }
                } else {
                    if large_coords.is_some() {
                        large_coords = None;
                    }
                }
                
                ui.add_space(PADDING);
                ui.label("requires a TISSUE_MIN_LENGTH above.");
                ui.label("requires a CONDITION_TISSUE_LAYER above.");
            }
            Condition::Default => {
                ui.label("Select a condition type.");
            }
            _ => {
                ui.label("Select a condition type.\n\n(This condition type is unsupported.\nIf you think this is an error please report it.)");
                ui.hyperlink_to(
                    "DF Graphics Helper on GitHub",
                    "https://github.com/BarelyCreative/DF-graphics-helper/tree/main",
                );
            }
        }
        ui.add_space(PADDING);
    }
}

#[derive(Clone, Default, Debug)]
struct TilePage {
    name: String,     //file name of tile_set_file_name.txt
    tiles: Vec<Tile>, //set of tiles defined in this file
}
impl TilePage {
    fn new() -> TilePage {
        TilePage {
            name: String::from("(new)"),
            tiles: vec![Tile::new()],
        }
    }

    fn empty() -> TilePage {
        TilePage {
            name: String::new(),
            tiles:Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Tile {
    name: String,           //all-caps NAME of tile
    filename: String,       //file path of image.png
    image_size: [usize; 2], //size of image in pixels
    tile_size: [usize; 2],  //size of tile in pixels
}
impl Tile {
    fn new() -> Tile {
        Tile {
            name: "(new)".to_string(),
            filename: String::new(),
            image_size: [0, 0],
            tile_size: [32, 32],
        }
    }

    fn empty() -> Tile {
        Tile {
            name: String::new(),
            filename: String::new(),
            image_size: [0, 0],
            tile_size: [32, 32],
        }
    }
}

enum MainWindow {
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
struct DFGraphicsHelper {
    main_window: MainWindow,
    loaded_graphics: Graphics,
    path: path::PathBuf,
    tilepage_index: usize,
    tile_index: usize,
    creaturefile_index: usize,
    creature_index: usize,
    layer_set_index: usize,
    layer_group_index: usize,
    layer_index: usize,
    condition_index: usize,
    texture: Option<egui::TextureHandle>,
    preview_image: bool,
}
impl DFGraphicsHelper {
    fn new() -> Self {
        Self {
            main_window: MainWindow::DefaultMenu,
            loaded_graphics: Graphics::new(),
            path: path::PathBuf::from(r".\graphics"),
            tilepage_index: usize::default(),
            tile_index: usize::default(),
            creaturefile_index: usize::default(),
            creature_index: usize::default(),
            layer_set_index: usize::default(),
            layer_group_index: usize::default(),
            layer_index: usize::default(),
            condition_index: usize::default(),
            texture: None,
            preview_image: false,
        }
    }

    fn main_tree(&mut self, ui: &mut Ui, ctx: &Context) {
        if ui
            .add(egui::Label::new("Tile Pages").sense(Sense::click()))
            .clicked()
        {
            self.main_window = MainWindow::TilePageDefaultMenu;
        };
        for (i_tilepage, tilepage) in self.loaded_graphics.tilepages.iter_mut().enumerate() {
            egui::collapsing_header::CollapsingState::load_with_default_open(
                ctx,
                format!("tilepage{}", i_tilepage).into(),
                false,
            )
            .show_header(ui, |ui| {
                if ui
                    .add(
                        egui::Label::new(format!("Tile Page: {}", &tilepage.name))
                            .sense(Sense::click()),
                    )
                    .clicked()
                {
                    self.main_window = MainWindow::TilePageMenu;
                    self.tilepage_index = i_tilepage;
                };
            })
            .body(|ui| {
                for (i_tile, tile) in tilepage.tiles.iter_mut().enumerate() {
                    if ui
                        .add(egui::Label::new(&tile.name).sense(Sense::click()))
                        .clicked()
                    {
                        self.main_window = MainWindow::TileMenu;
                        self.tilepage_index = i_tilepage;
                        self.tile_index = i_tile;
                    };
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
            egui::collapsing_header::CollapsingState::load_with_default_open(
                ctx,
                format!("creature_file{}", i_file).into(),
                false,
            )
            .show_header(ui, |ui| {
                if ui
                    .add(
                        egui::Label::new(format!("File: {}", &creature_file.name))
                            .sense(Sense::click()),
                    )
                    .clicked()
                {
                    self.main_window = MainWindow::CreatureFileMenu;
                    self.creaturefile_index = i_file;
                };
            })
            .body(|ui| {
                for (i_creature, creature) in creature_file.creatures.iter_mut().enumerate() {
                    egui::collapsing_header::CollapsingState::load_with_default_open(
                        ctx,
                        format!("creature{}{}", i_file, i_creature).into(),
                        true,
                    )
                    .show_header(ui, |ui| {
                        if ui
                            .add(egui::Label::new(&creature.name).sense(Sense::click()))
                            .clicked()
                        {
                            self.main_window = MainWindow::CreatureMenu;
                            self.creaturefile_index = i_file;
                            self.creature_index = i_creature;
                        };
                    })
                    .body(|ui| {
                        for (i_layer_set, layer_set) in creature.graphics_type.iter_mut().enumerate() {
                            match layer_set {
                                LayerSet::Empty => {
                                    if ui.add(egui::Label::new("(empty)")
                                        .sense(Sense::click())).clicked()
                                        {
                                        self.main_window = MainWindow::LayerMenu;
                                        self.creaturefile_index = i_file;
                                        self.creature_index = i_creature;
                                        self.layer_set_index = i_layer_set;
                                    }
                                },
                                LayerSet::Layered(state, layer_groups) => {
                                    egui::collapsing_header::CollapsingState::load_with_default_open(ctx,
                                        format!("layer_group{}{}{}",
                                        i_file, i_creature, i_layer_set).into(),
                                        false)
                                        .show_header(ui, |ui|
                                        {
                                        if ui.add(egui::Label::new(
                                            format!("Set: {}", state.name()))
                                            .sense(Sense::click())).clicked()
                                            {
                                            self.main_window = MainWindow::LayerSetMenu;
                                            self.creaturefile_index = i_file;
                                            self.creature_index = i_creature;
                                            self.layer_set_index = i_layer_set;
                                        };
                                    })
                                        .body(|ui|
                                        {
                                        for (i_layer_group, layer_group) in layer_groups.iter_mut().enumerate() {
                                            egui::collapsing_header::CollapsingState::load_with_default_open(ctx,
                                                format!("layer_group{}{}{}{}",
                                                i_file, i_creature, i_layer_set, i_layer_group).into(),
                                                false)
                                                .show_header(ui, |ui|
                                                {
                                                if ui.add(egui::Label::new(
                                                    format!("Group: {}", &layer_group.name))
                                                    .sense(Sense::click())).clicked()
                                                    {
                                                    self.main_window = MainWindow::LayerGroupMenu;
                                                    self.creaturefile_index = i_file;
                                                    self.creature_index = i_creature;
                                                    self.layer_set_index = i_layer_set;
                                                    self.layer_group_index = i_layer_group;
                                                };
                                            })
                                                .body(|ui|
                                                {
                                                for (i_layer, layer) in layer_group.layers.iter_mut().enumerate() {
                                                    egui::collapsing_header::CollapsingState::load_with_default_open(ctx,
                                                        format!("layer{}{}{}{}{}",
                                                        i_file, i_creature, i_layer_set, i_layer_group, i_layer).into(),
                                                        false)
                                                        .show_header(ui, |ui|
                                                        {
                                                        if ui.add(egui::Label::new(
                                                            format!("{}", &layer.name))
                                                            .sense(Sense::click())).clicked()
                                                            {
                                                            self.main_window = MainWindow::LayerMenu;
                                                            self.creaturefile_index = i_file;
                                                            self.creature_index = i_creature;
                                                            self.layer_set_index = i_layer_set;
                                                            self.layer_group_index = i_layer_group;
                                                            self.layer_index = i_layer;
                                                        };
                                                    })
                                                        .body(|ui|
                                                        {
                                                        for (i_condition, condition) in layer.conditions.iter_mut().enumerate() {
                                                            if ui.add(egui::Label::new(condition.name())
                                                                .sense(Sense::click())).clicked()
                                                                {
                                                                self.main_window = MainWindow::ConditionMenu;
                                                                self.creaturefile_index = i_file;
                                                                self.creature_index = i_creature;
                                                                self.layer_set_index = i_layer_set; 
                                                                self.layer_group_index = i_layer_group;
                                                                self.layer_index = i_layer;
                                                                self.condition_index = i_condition;
                                                            }
                                                        }
                                                    });
                                                }
                                            });
                                        }
                                    });
                                },
                                LayerSet::Simple(simple_layers) => {
                                    for (i_layer, layer) in simple_layers.iter_mut().enumerate() {
                                        if ui.add(egui::Label::new(
                                            format!("\t[{}]\t{}",
                                            layer.state.name(),
                                            layer.sub_state.clone().unwrap_or_else(|| State::Custom("".to_string())).name()))
                                            .sense(Sense::click())).clicked()
                                            {
                                            self.main_window = MainWindow::LayerMenu;
                                            self.creaturefile_index = i_file;
                                            self.creature_index = i_creature;
                                            self.layer_index = i_layer;
                                        }
                                    }
                                },
                                LayerSet::Statue(simple_layers) => {
                                    for (i_layer, layer) in simple_layers.iter_mut().enumerate() {
                                        if ui.add(egui::Label::new(
                                            format!("\tStatue: [{}]\t{}",
                                            layer.state.name(),
                                            layer.sub_state.clone().unwrap_or_else(|| State::Custom("".to_string())).name()))
                                            .sense(Sense::click())).clicked()
                                            {
                                            self.main_window = MainWindow::LayerMenu;
                                            self.creaturefile_index = i_file;
                                            self.creature_index = i_creature;
                                            self.layer_index = i_layer;
                                        }
                                    }
                                },
                                _ => {ui.label("uh oh layer sets todo");}
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

    fn default_menu(&mut self, ui: &mut Ui) {
        ui.label("Welcome!");
        ui.separator();

        ui.label("You have probably reached this menu due to an unfinished or buggy feature, sorry :(");
        ui.hyperlink_to(
            "DF Graphics Helper on GitHub",
            "https://github.com/BarelyCreative/DF-graphics-helper/tree/main",
        );
    }

    fn tilepage_default_menu(&mut self, ui: &mut Ui) {
        ui.label("Tile Page Menu");
        ui.separator();

        if ui.small_button("New Tile Page").clicked() {
            self.loaded_graphics.tilepages.push(TilePage::new());
        }
    }

    fn creature_default_menu(&mut self, ui: &mut Ui) {
        ui.label("Creature File Menu");
        ui.separator();

        if ui.small_button("New Creature File").clicked() {
            self.loaded_graphics
                .creature_files
                .push(CreatureFile::new());
        }
    }

    fn creature_file_menu(&mut self, ui: &mut Ui) {
        ui.label("Creature File Menu");
        if self.loaded_graphics.creature_files.is_empty() {
            self.main_window = MainWindow::CreatureDefaultMenu;
        } else {
            let creaturefile = self
                .loaded_graphics
                .creature_files
                .get_mut(self.creaturefile_index)
                .unwrap();

            ui.separator();
            ui.text_edit_singleline(&mut creaturefile.name);
            ui.add_space(PADDING);

            if ui.button("New Creature").clicked() {
                creaturefile.creatures.push(Creature::new());
            }
            ui.add_space(PADDING);
            ui.add_space(PADDING);

            if ui.button("Delete").clicked() {
                self.loaded_graphics
                    .creature_files
                    .remove(self.creaturefile_index);
                if self.creaturefile_index > 0 {
                    self.creaturefile_index = self.creaturefile_index - 1;
                } else if self.loaded_graphics.creature_files.is_empty() {
                    self.main_window = MainWindow::CreatureDefaultMenu;
                } else {
                    self.creaturefile_index = 0;
                }
            }
        }
    }

    fn creature_menu(&mut self, ui: &mut Ui) {
        ui.label("Creature Menu");
        if self
            .loaded_graphics
            .creature_files
            .get(self.creaturefile_index)
            .unwrap()
            .creatures
            .is_empty()
        {
            if ui.small_button("Create Creature").clicked() {
                self.loaded_graphics
                    .creature_files
                    .get_mut(self.creaturefile_index)
                    .unwrap()
                    .creatures
                    .push(Creature::new())
            }
        } else {
            let creature = self
                .loaded_graphics
                .creature_files
                .get_mut(self.creaturefile_index)
                .unwrap()
                .creatures
                .get_mut(self.creature_index)
                .unwrap();

            ui.separator();
            ui.text_edit_singleline(&mut creature.name);
            ui.add_space(PADDING);

            // egui::ComboBox::from_label("Graphics type")
            //     .selected_text(creature.graphics_type.name())
            //     .show_ui(ui, |ui| {
            //         ui.selectable_value(
            //             &mut creature.graphics_type,
            //             String::from("(select)"),
            //             "(select)",
            //         );
            //         ui.selectable_value(
            //             &mut creature.graphics_type,
            //             String::from("Simple"),
            //             "Simple",
            //         );
            //         ui.selectable_value(
            //             &mut creature.graphics_type,
            //             String::from("Layered"),
            //             "Layered",
            //         );
            //         ui.selectable_value(
            //             &mut creature.graphics_type,
            //             String::from("Statue"),
            //             "Statue",
            //         );
            //     });
            // ui.add_space(PADDING);

            // if creature.graphics_type.eq("Layered") {
            //     let default = &mut LayerGroup::new();
            //     let layer_group = creature.layer_groups.first_mut().unwrap_or(default);

            //     egui::ComboBox::from_label("State")
            //         .selected_text(&layer_group.set_state)
            //         .show_ui(ui, |ui| {
            //             ui.selectable_value(
            //                 &mut layer_group.set_state,
            //                 String::from("(select)"),
            //                 "(select)",
            //             );
            //             ui.selectable_value(
            //                 &mut layer_group.set_state,
            //                 String::from("DEFAULT"),
            //                 "DEFAULT",
            //             );
            //             ui.selectable_value(
            //                 &mut layer_group.set_state,
            //                 String::from("CHILD"),
            //                 "CHILD",
            //             );
            //             ui.selectable_value(
            //                 &mut layer_group.set_state,
            //                 String::from("CORPSE"),
            //                 "CORPSE",
            //             );
            //             ui.selectable_value(
            //                 &mut layer_group.set_state,
            //                 String::from("ANIMATED"),
            //                 "ANIMATED",
            //             );
            //             ui.selectable_value(
            //                 &mut layer_group.set_state,
            //                 String::from("LIST_ICON"),
            //                 "LIST_ICON",
            //             );
            //         });
            //     ui.horizontal(|ui| {
            //         ui.label("Custom State:");
            //         ui.text_edit_singleline(&mut layer_group.set_state);
            //     });
            //     ui.add_space(PADDING);
            // }

            // if ui.button("New Layer Group").clicked() {
            //     creature.layer_groups.push(LayerGroup::new());
            // }

            ui.add_space(PADDING);
            ui.add_space(PADDING);
            if ui.button("Delete").clicked() {
                self.loaded_graphics
                    .creature_files
                    .get_mut(self.creaturefile_index)
                    .unwrap()
                    .creatures
                    .remove(self.creature_index);

                if self.creature_index > 0 {
                    self.creature_index = self.creature_index - 1;
                } else if self
                    .loaded_graphics
                    .creature_files
                    .get(self.creaturefile_index)
                    .unwrap()
                    .creatures
                    .is_empty()
                {
                    self.main_window = MainWindow::CreatureFileMenu;
                } else {
                    self.creature_index = 0;
                }
            }
        }
    }

    fn tilepage_menu(&mut self, ui: &mut Ui) {
        ui.label("Tile Page Menu");
        if self.loaded_graphics.tilepages.is_empty() {
            self.main_window = MainWindow::TilePageDefaultMenu;
        } else {
            let tilepage = self
                .loaded_graphics
                .tilepages
                .get_mut(self.tilepage_index)
                .unwrap();

            ui.separator();
            ui.text_edit_singleline(&mut tilepage.name);
            ui.add_space(PADDING);

            if ui.button("New Tile").clicked() {
                tilepage.tiles.push(Tile::new());
            }
            ui.add_space(PADDING);
            ui.add_space(PADDING);

            if ui.button("Delete").clicked() {
                self.loaded_graphics.tilepages.remove(self.tilepage_index);
                if self.tilepage_index > 0 {
                    self.tilepage_index = self.tilepage_index - 1;
                } else if self.loaded_graphics.tilepages.is_empty() {
                    self.main_window = MainWindow::TilePageDefaultMenu;
                } else {
                    self.tilepage_index = 0;
                }
            }
        }
    }

    fn tile_menu(&mut self, ui: &mut Ui) {
        ui.label("Tile Menu");
        if self
            .loaded_graphics
            .tilepages
            .get(self.tilepage_index)
            .unwrap()
            .tiles
            .is_empty()
        {
            if ui.small_button("Create Tile").clicked() {
                self.loaded_graphics
                    .tilepages
                    .get_mut(self.tilepage_index)
                    .unwrap()
                    .tiles
                    .push(Tile::new());
            }
        } else {
            let tile = self
                .loaded_graphics
                .tilepages
                .get_mut(self.tilepage_index)
                .unwrap()
                .tiles
                .get_mut(self.tile_index)
                .unwrap();

            ui.separator();
            ui.label("Tile token");
            ui.text_edit_singleline(&mut tile.name);
            ui.add_space(PADDING);

            ui.label("Image file path");
            ui.horizontal(|ui| {
                ui.label("/graphics/images/");
                ui.text_edit_singleline(&mut tile.filename);
                ui.label(".png");
            });
            ui.add_space(PADDING);

            ui.label("Image size (automatic todo)");
            ui.horizontal(|ui| {
                ui.label(format!("Width: {}", tile.image_size[0]));
                ui.label(format!("Height: {}", tile.image_size[1]));
            });
            ui.add_space(PADDING);

            ui.label("Tile size (pixels)");
            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(&mut tile.tile_size[0], 0..=64).prefix("Width: "));
                ui.add(egui::Slider::new(&mut tile.tile_size[1], 0..=96).prefix("Height: "));
            });
            ui.add_space(PADDING);

            self.preview_image(ui, None);

            ui.add_space(PADDING);
            ui.add_space(PADDING);

            if ui.button("Delete").clicked() {
                self.loaded_graphics
                    .tilepages
                    .get_mut(self.tilepage_index)
                    .unwrap()
                    .tiles
                    .remove(self.tile_index);

                if self.tile_index > 0 {
                    self.tile_index = self.tile_index - 1;
                } else if self
                    .loaded_graphics
                    .tilepages
                    .get(self.tilepage_index)
                    .unwrap()
                    .tiles
                    .is_empty()
                {
                    self.main_window = MainWindow::TilePageMenu;
                } else {
                    self.tile_index = 0;
                }
            }
        }
    }

    fn layer_group_menu(&mut self, ui: &mut Ui) {
        ui.label("Layer Group Menu");
        // if self
        //     .loaded_graphics
        //     .creature_files
        //     .get(self.creaturefile_index)
        //     .unwrap()
        //     .creatures
        //     .get(self.creature_index)
        //     .unwrap()
        //     .layer_groups
        //     .is_empty()
        // {
        //     if ui.small_button("Create Layer Group").clicked() {
        //         self.loaded_graphics
        //             .creature_files
        //             .get_mut(self.creaturefile_index)
        //             .unwrap()
        //             .creatures
        //             .get_mut(self.creature_index)
        //             .unwrap()
        //             .layer_groups
        //             .push(LayerGroup::new())
        //     }
        // } else {
        //     let creature = self
        //         .loaded_graphics
        //         .creature_files
        //         .get_mut(self.creaturefile_index)
        //         .unwrap()
        //         .creatures
        //         .get_mut(self.creature_index)
        //         .unwrap();
        //     let layer_group = creature
        //         .layer_groups
        //         .get_mut(self.layer_group_index)
        //         .unwrap();

        //     ui.separator();
        //     ui.label("Layer group name:");
        //     ui.text_edit_singleline(&mut layer_group.name);
        //     ui.add_space(PADDING);

        //     if creature.graphics_type.eq("Layered") {
        //         if ui.button("New Layer").clicked() {
        //             layer_group.layers.push(Layer::new());
        //         }
        //     } else {
        //         self.layer_menu(ui);
        //     }

        //     ui.add_space(PADDING);
        //     ui.add_space(PADDING);

        //     if ui.button("Delete").clicked() {
        //         self.loaded_graphics
        //             .creature_files
        //             .get_mut(self.creaturefile_index)
        //             .unwrap()
        //             .creatures
        //             .get_mut(self.creature_index)
        //             .unwrap()
        //             .layer_groups
        //             .remove(self.layer_group_index);

        //         if self.layer_group_index > 0 {
        //             self.layer_group_index = self.layer_group_index - 1;
        //         } else if self
        //             .loaded_graphics
        //             .creature_files
        //             .get(self.creaturefile_index)
        //             .unwrap()
        //             .creatures
        //             .get(self.creature_index)
        //             .unwrap()
        //             .layer_groups
        //             .is_empty()
        //         {
        //             self.main_window = MainWindow::CreatureMenu;
        //         } else {
        //             self.layer_group_index = 0;
        //         }
        //     }
        // }
    }

    fn layer_menu(&mut self, ui: &mut Ui) {
        //Layer { conditions: todo!() };
        ui.label("Layer Menu");
        // if self
        //     .loaded_graphics
        //     .creature_files
        //     .get(self.creaturefile_index)
        //     .unwrap()
        //     .creatures
        //     .get(self.creature_index)
        //     .unwrap()
        //     .layer_groups
        //     .get(self.layer_group_index)
        //     .unwrap()
        //     .layers
        //     .is_empty()
        // {
        //     //if there are no layers defined show create layer button only
        //     if ui.small_button("Create Layer").clicked() {
        //         self.loaded_graphics
        //             .creature_files
        //             .get_mut(self.creaturefile_index)
        //             .unwrap()
        //             .creatures
        //             .get_mut(self.creature_index)
        //             .unwrap()
        //             .layer_groups
        //             .get_mut(self.layer_group_index)
        //             .unwrap()
        //             .layers
        //             .push(Layer::new())
        //     }
        // } else {
        //     //show standard layer menu
        //     let creature = self
        //         .loaded_graphics
        //         .creature_files
        //         .get_mut(self.creaturefile_index)
        //         .unwrap()
        //         .creatures
        //         .get_mut(self.creature_index)
        //         .unwrap();
        //     let layer_group = creature
        //         .layer_groups
        //         .get_mut(self.layer_group_index)
        //         .unwrap();
        //     let layer = layer_group.layers.get_mut(self.layer_index).unwrap();
        //     let mut large;

        //     if layer.coords.eq(&layer.large_coords) {
        //         large = false;
        //     } else {
        //         large = true;
        //     }

        //     ui.separator();
        //     if creature.graphics_type.eq("Layered") {
        //         //for layered graphics no state
        //         ui.label("Layer name:");
        //         ui.text_edit_singleline(&mut layer.name);
        //     } else {
        //         //for simple or statue layers show state selections for layer_group
        //         egui::ComboBox::from_label("State")
        //             .selected_text(layer_group.set_state.as_str())
        //             .show_ui(ui, |ui| {
        //                 ui.selectable_value(
        //                     &mut layer_group.set_state,
        //                     String::from("(select)"),
        //                     "(select)",
        //                 );
        //                 ui.selectable_value(
        //                     &mut layer_group.set_state,
        //                     String::from("DEFAULT"),
        //                     "DEFAULT",
        //                 );
        //                 ui.selectable_value(
        //                     &mut layer_group.set_state,
        //                     String::from("CHILD"),
        //                     "CHILD",
        //                 );
        //                 ui.selectable_value(
        //                     &mut layer_group.set_state,
        //                     String::from("CORPSE"),
        //                     "CORPSE",
        //                 );
        //                 ui.selectable_value(
        //                     &mut layer_group.set_state,
        //                     String::from("ANIMATED"),
        //                     "ANIMATED",
        //                 );
        //                 ui.selectable_value(
        //                     &mut layer_group.set_state,
        //                     String::from("LIST_ICON"),
        //                     "LIST_ICON",
        //                 );
        //             });
        //         // let state = layer
        //         //     .conditions
        //         //     .first_mut()
        //         //     .unwrap()
        //         //     .cond_type
        //         //     .name();
        //         // egui::ComboBox::from_label("Secondary State (optional)")
        //         //     .selected_text(state.as_str())
        //         //     .show_ui(ui, |ui| {
        //         //         ui.selectable_value(state, String::from(""), "(none)");
        //         //         ui.selectable_value(state, String::from("DEFAULT"), "DEFAULT");
        //         //         ui.selectable_value(state, String::from("CHILD"), "CHILD");
        //         //         ui.selectable_value(state, String::from("CORPSE"), "CORPSE");
        //         //         ui.selectable_value(state, String::from("ANIMATED"), "ANIMATED");
        //         //         ui.selectable_value(state, String::from("LIST_ICON"), "LIST_ICON");
        //         //     });
        //     }
        //     ui.add_space(PADDING);

        //     egui::ComboBox::from_label("Tile")
        //         .selected_text(&layer.tile)
        //         .show_ui(ui, |ui| {
        //             ui.selectable_value(&mut layer.tile, String::from("(select)"), "(select)");
        //             for tilepage in self.loaded_graphics.tilepages.iter_mut() {
        //                 for tile in tilepage.tiles.iter_mut() {
        //                     ui.selectable_value(
        //                         &mut layer.tile,
        //                         tile.name.to_string(),
        //                         tile.name.to_string(),
        //                     );
        //                 }
        //             }
        //         });
        //     for (i, tilepage) in self.loaded_graphics.tilepages.iter().enumerate() {
        //         for (j, tile) in tilepage.tiles.iter().enumerate() {
        //             if &tile.name == &layer.tile {
        //                 self.tilepage_index = i;
        //                 self.tile_index = j;
        //             }
        //         }
        //     }
        //     ui.horizontal(|ui| {
        //         ui.label("New Tile:");
        //         ui.text_edit_singleline(&mut layer.tile);
        //         if ui.small_button("Save").clicked() {
        //             if self.loaded_graphics.tilepages.is_empty() {
        //                 self.loaded_graphics.tilepages.push(TilePage::new());
        //             }
        //             self.loaded_graphics
        //                 .tilepages
        //                 .last_mut()
        //                 .unwrap()
        //                 .tiles
        //                 .push(Tile {
        //                     name: layer.tile.to_string(),
        //                     ..Default::default()
        //                 });
        //         }
        //     });
        //     ui.add_space(PADDING);

        //     ui.label("Upper left coordinates (tiles)");
        //     ui.horizontal(|ui| {
        //         ui.add(
        //             egui::DragValue::new(&mut layer.coords[0])
        //                 .speed(1)
        //                 .clamp_range(0..=usize::MAX)
        //                 .prefix("X: "),
        //         );
        //         ui.add(
        //             egui::DragValue::new(&mut layer.coords[1])
        //                 .speed(1)
        //                 .clamp_range(0..=usize::MAX)
        //                 .prefix("Y: "),
        //         );
        //     });
        //     ui.horizontal(|ui| {
        //         ui.label("Large Image:");
        //         if ui.checkbox(&mut large, "").changed() {
        //             if large {
        //                 layer.large_coords = [layer.coords[0] + 1, layer.coords[1] + 1];
        //             } else {
        //                 layer.large_coords = [layer.coords[0], layer.coords[1]];
        //             }
        //         }
        //     });

        //     if large {
        //         ui.horizontal(|ui| {
        //             ui.add(
        //                 egui::Slider::new(
        //                     &mut layer.large_coords[0],
        //                     layer.coords[0]..=layer.coords[0] + 2,
        //                 )
        //                 .prefix("X: "),
        //             );
        //             ui.add(
        //                 egui::Slider::new(
        //                     &mut layer.large_coords[1],
        //                     layer.coords[1]..=layer.coords[1] + 1,
        //                 )
        //                 .prefix("Y: "),
        //             );
        //         });
        //     }
        //     ui.add_space(PADDING);

        //     let rect = Some([layer.coords, layer.large_coords]);
        //     self.preview_image(ui, rect);

        //     ui.add_space(PADDING);
        //     ui.add_space(PADDING);
        //     let creature = self
        //         .loaded_graphics
        //         .creature_files
        //         .get_mut(self.creaturefile_index)
        //         .unwrap()
        //         .creatures
        //         .get_mut(self.creature_index)
        //         .unwrap();
        //     if creature.graphics_type.eq("Layered") {
        //         if ui.button("Delete").clicked() {
        //             self.loaded_graphics
        //                 .creature_files
        //                 .get_mut(self.creaturefile_index)
        //                 .unwrap()
        //                 .creatures
        //                 .get_mut(self.creature_index)
        //                 .unwrap()
        //                 .layer_groups
        //                 .get_mut(self.layer_group_index)
        //                 .unwrap()
        //                 .layers
        //                 .remove(self.layer_index);

        //             if self.layer_index > 0 {
        //                 self.layer_index = self.layer_index - 1;
        //             } else if self
        //                 .loaded_graphics
        //                 .creature_files
        //                 .get(self.creaturefile_index)
        //                 .unwrap()
        //                 .creatures
        //                 .get(self.creature_index)
        //                 .unwrap()
        //                 .layer_groups
        //                 .get(self.layer_group_index)
        //                 .unwrap()
        //                 .layers
        //                 .is_empty()
        //             {
        //                 self.main_window = MainWindow::LayerGroupMenu;
        //             } else {
        //                 self.layer_index = 0;
        //             }
        //         }
        //     } else {
        //         if ui.button("Delete").clicked() {
        //             self.loaded_graphics
        //                 .creature_files
        //                 .get_mut(self.creaturefile_index)
        //                 .unwrap()
        //                 .creatures
        //                 .get_mut(self.creature_index)
        //                 .unwrap()
        //                 .layer_groups
        //                 .remove(self.layer_group_index);

        //             if self.layer_group_index > 0 {
        //                 self.layer_group_index = self.layer_group_index - 1;
        //             } else if self
        //                 .loaded_graphics
        //                 .creature_files
        //                 .get(self.creaturefile_index)
        //                 .unwrap()
        //                 .creatures
        //                 .get(self.creature_index)
        //                 .unwrap()
        //                 .layer_groups
        //                 .is_empty()
        //             {
        //                 self.main_window = MainWindow::CreatureMenu;
        //             } else {
        //                 self.layer_group_index = 0;
        //             }
        //         }
        //     }
        // }
    }

    fn preview_image(&mut self, ui: &mut Ui, rect: Option<[[usize; 2]; 2]>) {
        let tile = self
            .loaded_graphics
            .tilepages
            .get_mut(self.tilepage_index)
            .unwrap()
            .tiles
            .get_mut(self.tile_index)
            .unwrap();

        ui.horizontal(|ui| {
            ui.checkbox(&mut self.preview_image, "View Image"); //determine if preview image is desired
            if ui.button("Refresh").clicked() {
                //clear image if stale
                self.texture = None;
            }
        });

        if self.preview_image && self.texture.is_some() {
            //display texture once loaded
            let texture: &egui::TextureHandle = self.texture.as_ref().unwrap();
            let size = texture.size_vec2();

            let image =
                PlotImage::new(texture, PlotPoint::new(size[0] / 2.0, size[1] / -2.0), size);

            let x_fmt = |x, _range: &std::ops::RangeInclusive<f64>| {
                if x < 0.0 {
                    // No labels outside value bounds
                    String::new()
                } else {
                    // Tiles
                    format!("{}", (x as f64 / 32.0).floor())
                }
            };

            let y_fmt = |y, _range: &std::ops::RangeInclusive<f64>| {
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
                    "{x}, {y}",
                    x = (val.x / 32.0).floor(),
                    y = (val.y / -32.0).floor()
                )
            };

            let plot = Plot::new("image_preview")
                .auto_bounds_x()
                .auto_bounds_y()
                .data_aspect(1.0)
                .show_background(true)
                .set_margin_fraction(egui::vec2(0.03, 0.03))
                .x_grid_spacer(Self::grid)
                .y_grid_spacer(Self::grid)
                .x_axis_formatter(x_fmt)
                .y_axis_formatter(y_fmt)
                .label_formatter(label_fmt);
            plot.show(ui, |plot_ui| {
                plot_ui.image(image.name("Image"));
                if rect.is_some() {
                    let rect = rect.unwrap();

                    let points = vec![
                        [rect[0][0] as f64 * 32.0, rect[0][1] as f64 * -32.0],
                        [rect[1][0] as f64 * 32.0 + 32.0, rect[0][1] as f64 * -32.0],
                        [
                            rect[1][0] as f64 * 32.0 + 32.0,
                            rect[1][1] as f64 * -32.0 - 32.0,
                        ],
                        [rect[0][0] as f64 * 32.0, rect[1][1] as f64 * -32.0 - 32.0],
                    ];

                    let rectangle = egui::plot::Polygon::new(points);
                    plot_ui.polygon(rectangle);
                }
            });
        } else if self.preview_image && self.texture.is_none() {
            //load texture from path
            let imagepath: path::PathBuf = format!(
                "{}\\images\\{}.png",
                self.path.to_str().unwrap(),
                &tile.filename
            )
            .into();
            if imagepath.exists() {
                let image = image::io::Reader::open(imagepath)
                    .unwrap()
                    .decode()
                    .unwrap();
                let size = [image.width() as _, image.height() as _];
                let image_buffer = image.to_rgba8();
                let pixels = image_buffer.as_flat_samples();
                let rgba = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());

                tile.image_size = size.clone();
                self.texture.get_or_insert_with(|| {
                    ui.ctx()
                        .load_texture("default_image", rgba, Default::default())
                });
            }
        }
    }

    fn grid(input: egui::plot::GridInput) -> Vec<egui::plot::GridMark> {
        // Note: this always fills all possible marks. For optimization, `input.bounds`
        // could be used to decide when the low-interval grids (minutes) should be added.

        let mut marks = vec![];

        let (min, max) = input.bounds;
        let min = min.floor() as i32;
        let max = max.ceil() as i32;

        for i in min..=max {
            let step_size = if i % 3200.0 as i32 == 0 {
                // 100 tile
                3200.0
            } else if i % 320.0 as i32 == 0 {
                // 10 tile
                320.0
            } else if i % 32.0 as i32 == 0 {
                // 1 tile
                32.0
            } else if i % 1.0 as i32 == 0 {
                // 1 px
                1.0
            } else {
                // skip grids below 5min
                continue;
            };

            marks.push(egui::plot::GridMark {
                value: i as f64,
                step_size,
            });
        }

        marks
    }

    fn condition_menu(&mut self, ui: &mut Ui) {
        ui.label("Condition Menu");
        // if self
        //     .loaded_graphics
        //     .creature_files
        //     .get(self.creaturefile_index)
        //     .unwrap()
        //     .creatures
        //     .get(self.creature_index)
        //     .unwrap()
        //     .layer_groups
        //     .get(self.layer_group_index)
        //     .unwrap()
        //     .layers
        //     .get(self.condition_index)
        //     .unwrap()
        //     .conditions
        //     .is_empty()
        // {
        //     //if there are no layers defined show create layer button only
        //     if ui.small_button("New condition").clicked() {
        //         self.loaded_graphics
        //             .creature_files
        //             .get_mut(self.creaturefile_index)
        //             .unwrap()
        //             .creatures
        //             .get_mut(self.creature_index)
        //             .unwrap()
        //             .layer_groups
        //             .get_mut(self.layer_group_index)
        //             .unwrap()
        //             .layers
        //             .get_mut(self.layer_index)
        //             .unwrap()
        //             .conditions
        //             .push(Condition::new())
        //     }
        // } else {
        //     let condition = self
        //         .loaded_graphics
        //         .creature_files
        //         .get_mut(self.creaturefile_index)
        //         .unwrap()
        //         .creatures
        //         .get_mut(self.creature_index)
        //         .unwrap()
        //         .layer_groups
        //         .get_mut(self.layer_group_index)
        //         .unwrap()
        //         .layers
        //         .get_mut(self.layer_index)
        //         .unwrap()
        //         .conditions
        //         .get_mut(self.condition_index)
        //         .unwrap();

        //     ui.separator();

        //     let mut tile_names: Vec<String> = self
        //         .loaded_graphics
        //         .tilepages
        //         .iter()
        //         .flat_map(|tilepage| tilepage.tiles.iter().map(|tile| tile.name.to_string()))
        //         .collect();
        //     tile_names.sort();
        //     tile_names.dedup();

        //     condition.condition_menu(ui, &tile_names);

        //     ui.add_space(PADDING);
        //     if ui.button("Delete").clicked() {
        //         self.loaded_graphics
        //             .creature_files
        //             .get_mut(self.creaturefile_index)
        //             .unwrap()
        //             .creatures
        //             .get_mut(self.creature_index)
        //             .unwrap()
        //             .layer_groups
        //             .get_mut(self.layer_group_index)
        //             .unwrap()
        //             .layers
        //             .get_mut(self.layer_index)
        //             .unwrap()
        //             .conditions
        //             .remove(self.condition_index);
        //         if self.condition_index > 0 {
        //             self.condition_index = self.condition_index - 1;
        //         } else if self
        //             .loaded_graphics
        //             .creature_files
        //             .get(self.creaturefile_index)
        //             .unwrap()
        //             .creatures
        //             .get(self.creature_index)
        //             .unwrap()
        //             .layer_groups
        //             .get(self.layer_group_index)
        //             .unwrap()
        //             .layers
        //             .get(self.layer_index)
        //             .unwrap()
        //             .conditions
        //             .is_empty()
        //         {
        //             self.main_window = MainWindow::LayerMenu;
        //         } else {
        //             self.condition_index = 0;
        //         }
        //     }
        // }
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
                    if ui.button("Set Path..").clicked() {
                        self.path = rfd::FileDialog::new()
                            .set_title(r"Choose graphics folder")
                            .pick_folder()
                            .unwrap_or(path::PathBuf::from(r".\graphics"));
                        self.loaded_graphics = Graphics::import(self.path.clone());
                        ui.close_menu();
                    }
                    if ui.button("Import").clicked() {
                        self.loaded_graphics = Graphics::import(self.path.clone());
                        ui.close_menu();
                    }
                    if ui.button("Export").clicked() {
                        self.loaded_graphics.export();
                        ui.close_menu();
                    }
                });
            });
        });

        egui::SidePanel::new(egui::panel::Side::Left, "tree")
            .resizable(true)
            .show(ctx, |ui| {
                //Draw tree-style selection menu on left side
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.main_tree(ui, ctx);
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            //Draw main window by matching self.main_window
            egui::ScrollArea::both().show(ui, |ui| match self.main_window {
                MainWindow::TilePageDefaultMenu => self.tilepage_default_menu(ui),
                MainWindow::TilePageMenu => self.tilepage_menu(ui),
                MainWindow::TileMenu => self.tile_menu(ui),
                MainWindow::CreatureDefaultMenu => self.creature_default_menu(ui),
                MainWindow::CreatureFileMenu => self.creature_file_menu(ui),
                MainWindow::CreatureMenu => self.creature_menu(ui),
                MainWindow::LayerSetMenu => self.default_menu(ui),
                MainWindow::LayerGroupMenu => self.layer_group_menu(ui),
                MainWindow::LayerMenu => self.layer_menu(ui),
                MainWindow::ConditionMenu => self.condition_menu(ui),
                MainWindow::ReferenceMenu => self.default_menu(ui),
                MainWindow::DefaultMenu => self.default_menu(ui),
            });
        });
    }
}

fn main() {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        resizable: true,
        maximized: true,
        follow_system_theme: true,
        default_theme: eframe::Theme::Dark,
        ..Default::default()
    };
    eframe::run_native(
        "DF Graphics Helper",
        options,
        Box::new(|_cc| Box::new(DFGraphicsHelper::new())),
    )
    .expect("should always run");
}
