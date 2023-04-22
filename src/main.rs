// #![windows_subsystem = "windows"]

use egui::plot::{Plot, PlotImage, PlotPoint};
use egui::{Context, Sense, Ui};
use convert_case::{Boundary, Case, Casing};
use rfd;
use std::error::Error;
use std::io::prelude::*;
use std::{fs, io, path};

const PADDING: f32 = 8.0;

#[derive(Clone, Default)]
struct Graphics {
    tile_pages: Vec<TilePage>,
    creature_files: Vec<CreatureFile>,
}
impl Graphics {
    fn new() -> Graphics {
        Graphics {
            tile_pages: vec![TilePage::new()],
            creature_files: vec![CreatureFile::new()],
        }
    }

    fn read_brackets(raw_line: &String) -> (bool, Vec<String>, Option<String>) {
        let brackets = raw_line.contains("[") && raw_line.contains("]");
        let line_vec: Vec<String>;
        let comments: Option<String>;
        if brackets {
            // let clean_line = raw_line.replace("[", "").replace("]", "").trim().to_string();
            let start_line = raw_line.trim().replace("[", "");
            let mut split_line = start_line.split("]");

            line_vec = split_line.next().unwrap().split(":").map(|s| s.to_string()).collect();
            if let Some(c) = split_line.next() {
                comments = Some(c.trim().to_string());
            } else {
                comments = None;
            }
        } else {
            line_vec = Vec::new();
            comments = None;
        }

        (brackets, line_vec, comments) //retain first bracket to ignore commented-out lines
    }

    fn import(folder: &mut path::PathBuf) -> Result<(Graphics, path::PathBuf), Box<dyn Error>> {
        let mut tile_pages: Vec<TilePage> = Vec::new();
        let mut creature_files: Vec<CreatureFile> = Vec::new();

        if folder.ends_with("graphics") {
            folder.pop();
        } else if folder.ends_with("images") && folder.parent().get_or_insert(path::Path::new("")).ends_with("graphics") {
            folder.pop();
            folder.pop();
        } else if !folder.read_dir()?.any(|f| if f.is_ok() {f.unwrap().path().ends_with("graphics")} else {false}) {
            panic!("no /graphics directory found");
        }

        let paths = fs::read_dir(&folder.join("graphics")).expect("expects 0 or more valid file paths"); //read graphics directory

        for path in paths {
            let path = path?;
            let entry_name = path.file_name().into_string().expect("utf-8 string");//only supports utf-8 file paths

            if entry_name.ends_with(".txt") {
                if entry_name.starts_with("tile_page_") {
                    //read tile page file
                    tile_pages.push(Self::import_tile_page(folder, &path)?);

                } else if entry_name.starts_with("graphics_creatures_") {
                    //read creature file
                    creature_files.push(Self::import_creature_file(&path)?);

                    //naming the layer groups
                    Self::rename_layer_groups(&mut creature_files);
                }
            }
        }

        Ok(
            (Graphics {
                tile_pages: tile_pages,
                creature_files: creature_files,
                ..Default::default()
            },
            folder.clone())
        )
    }

    fn import_tile_page(folder: &mut path::PathBuf, path: &fs::DirEntry) -> Result<TilePage, Box<dyn Error>> {
        let mut tile_page = TilePage::empty();
        let mut tile = Tile::empty();

        let f =
            fs::File::open(path.path()).expect("failed to open tile page file");
        
        for raw_line_result in io::BufReader::new(f).lines() {
            //read line-by-line
            let raw_line = raw_line_result.expect("the for loop should always give a valid String");
            let (brackets, line_vec, _) = Self::read_brackets(&raw_line);

            if tile_page.name.is_empty() {
                tile_page.name = raw_line.replace("tile_page_", "").trim().to_string();

            } else if brackets && line_vec.len() > 0 {
                match line_vec[0].as_str() {
                    "TILE_PAGE" => {
                        if !tile.name.is_empty() {
                            tile_page.tiles.push(tile.clone());
                            tile = Tile::empty();
                            tile.name.clear();
                        }
                        tile.name = line_vec[1].clone();
                    },
                    "FILE" => {
                    tile.filename = line_vec[1].clone()
                        .replace(".png", "")
                        .replace("images", "")
                        .replace("/", "")
                        .replace("/", "");
                    },
                    "TILE_DIM" => {
                        tile.tile_size = 
                            [line_vec[1].parse().unwrap_or_default(),
                            line_vec[2].parse().unwrap_or_default()];
                    },
                    "PAGE_DIM_PIXELS" => {
                        let image_path: path::PathBuf = folder
                            .join("images")
                            .join(format!("{}.png", tile.filename));

                        if let Ok((x, y)) = image::image_dimensions(image_path) {
                            tile.image_size = [x, y];
                        } else {
                            tile.image_size = 
                                [line_vec[1].parse().unwrap_or_default(),
                                line_vec[2].parse().unwrap_or_default()];
                        }
                    },
                    _ => {},
                }
            }
        }
        tile_page.tiles.push(tile);

        Ok(tile_page)
    }

    fn import_creature_file(path: &fs::DirEntry) -> Result<CreatureFile, Box<dyn Error>> {
        let mut creature_file = CreatureFile::empty();
        let mut creature = Creature::empty();
        let mut layer_set = LayerSet::default();
        let mut layer_group = LayerGroup::empty();
        let mut simple_layer = SimpleLayer::empty();
        let mut layer = Layer::empty();
        let mut condition = Condition::default();

        let f =
            fs::File::open(path.path()).expect("failed to open creature graphics file");

        for raw_line_result in io::BufReader::new(f).lines() {

            let raw_line = raw_line_result.unwrap();
            let (brackets, mut line_vec, comments) = Self::read_brackets(&raw_line);
            // let line_vec = line_vec_as_is.drain_filter(|string| string.as_ref() == "AS_IS").collect();
            line_vec.retain(|elem| elem != "AS_IS");//remove any AS_IS elements there are for creature files due to not palletization(?) v50.05

            if creature_file.name.is_empty() {
                //set creature file name
                creature_file.name =
                    raw_line.replace("graphics_creatures_", "").trim().to_string();
            } else if brackets && line_vec.len() > 0 {

                // dbg!(&raw_line);

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
                                if layer.name.ne("") {// !layer_groups.is_empty() | 
                                    if condition.ne(&Condition::default()) {
                                        layer.conditions.push(condition.clone());
                                        condition = Condition::default();
                                    }
                                    layer_group.layers.push(layer.clone());
                                    layer = Layer::empty();
                                    layer_groups.push(layer_group.clone());
                                    layer_group = LayerGroup::empty();
                                }
                            },
                            LayerSet::Simple(simple_layers) | LayerSet::Statue(simple_layers) => {
                                if simple_layer.state.ne(&State::default()) {
                                    simple_layers.push(simple_layer);
                                    simple_layer = SimpleLayer::empty();
                                }
                            },
                            // _ => {panic!()}
                        }
                        match line_vec[0].as_str() {
                            //set up the buffered graphics according to the current line
                            "CREATURE_GRAPHICS" => {
                                if creature.name.ne("") {
                                    creature.graphics_type.push(layer_set.clone());
                                    creature_file.creatures.push(creature.clone());
                                    creature.graphics_type.clear();
                                }
                                creature.name = line_vec[1].clone();
                                layer_set = LayerSet::Simple(Vec::new());
                            },
                            "STATUE_CREATURE_GRAPHICS" => {
                                if creature.name.ne("") {
                                    creature.graphics_type.push(layer_set.clone());
                                    creature_file.creatures.push(creature.clone());
                                    creature.graphics_type.clear();
                                }
                                creature.name = line_vec[1].clone();
                                layer_set = LayerSet::Statue(Vec::new());
                            },
                            "LAYER_SET" => {
                                if creature.name.ne("") {
                                    match &layer_set {
                                        LayerSet::Simple(simple_layers) => {
                                            if !simple_layers.is_empty() {
                                                creature.graphics_type.push(layer_set.clone());
                                            }
                                        },
                                        _ => {creature.graphics_type.push(layer_set.clone());}
                                    }
                                }
                                layer_set = LayerSet::Layered(
                                    State::from(line_vec[1].clone()),
                                    Vec::new()
                                );
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
                                    condition = Condition::default();
                                    layer_group.layers.push(layer.clone());
                                    layer = Layer::empty();
                                    layer_groups.push(layer_group);
                                    layer_group = LayerGroup::empty();
                                }
                            },
                            _ => { panic!("should not see layer groups outside of a 'layered' layer set"); }
                        }
                        if let Some(c) = comments {
                            layer_group.name = c.replace("---", "");
                        }
                    },
                    "LAYER" => {
                        match &mut layer_set {
                            LayerSet::Layered(..) => {
                                if layer.name.ne("") {
                                    if condition.ne(&Condition::default()) {
                                        layer.conditions.push(condition.clone());
                                        condition = Condition::default();
                                    }
                                    layer_group.layers.push(layer.clone());
                                }
                                if line_vec[3].eq("LARGE_IMAGE") {
                                    let c = [line_vec[4].parse::<u32>().unwrap_or_default(),
                                        line_vec[5].parse::<u32>().unwrap_or_default()];
                                    let l_c = [line_vec[6].parse::<u32>().unwrap_or_default(),
                                        line_vec[7].parse::<u32>().unwrap_or_default()];
                                    let large;
                                    if (c[0] <= l_c[0]) && (c[1] <= l_c[1]) {
                                        large = [l_c[0]-c[0], l_c[1]-c[1]];
                                    } else {
                                        panic!("impossible large coordinates import at {}", raw_line.trim())
                                    }
                                    layer = Layer{
                                        name: line_vec[1].clone(),
                                        conditions: Vec::new(),
                                        tile: line_vec[2].clone(),
                                        coords: c,
                                        large_coords: Some(large),
                                    }
                                } else {
                                    layer = Layer{
                                        name: line_vec[1].clone(),
                                        conditions: Vec::new(),
                                        tile: line_vec[2].clone(),
                                        coords:
                                            [line_vec[3].parse::<u32>().unwrap_or_default(),
                                            line_vec[4].parse::<u32>().unwrap_or_default()],
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
                                    let c = [line_vec[3].parse::<u32>().unwrap_or_default(),
                                        line_vec[4].parse::<u32>().unwrap_or_default()];
                                    let l_c = [line_vec[5].parse::<u32>().unwrap_or_default(),
                                        line_vec[6].parse::<u32>().unwrap_or_default()];
                                    let large;
                                    if (c[0] <= l_c[0]) && (c[1] <= l_c[1]) {
                                        large = [l_c[0]-c[0], l_c[1]-c[1]];
                                    } else {
                                        panic!("impossible large coordinates import at {}", raw_line.trim())
                                    }
                                    simple_layer = SimpleLayer {
                                        state: State::from(line_vec[0].clone()),
                                        tile: line_vec[1].clone(),
                                        coords: c,
                                        large_coords: Some(large),
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
                                            [line_vec[2].parse::<u32>().unwrap_or_default(),
                                            line_vec[3].parse::<u32>().unwrap_or_default()],
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
                                let c = [line_vec[2].parse::<u32>().unwrap_or_default(),
                                    line_vec[3].parse::<u32>().unwrap_or_default()];
                                let l_c = [line_vec[4].parse::<u32>().unwrap_or_default(),
                                    line_vec[5].parse::<u32>().unwrap_or_default()];
                                let large;
                                if (c[0] <= l_c[0]) && (c[1] <= l_c[1]) {
                                    large = [l_c[0]-c[0], l_c[1]-c[1]];
                                } else {
                                    panic!("impossible large coordinates import at {}", raw_line.trim())
                                }
                                simple_layer = SimpleLayer {
                                    state: State::from(line_vec[0].clone()),
                                    tile: line_vec[1].clone(),
                                    coords: c,
                                    large_coords: Some(large),
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
        if condition.ne(&Condition::default()) {
            layer.conditions.push(condition);
        }
        if layer.name.ne("") {
            layer_group.layers.push(layer);
        }
        match &mut layer_set {
            LayerSet::Empty => {},
            LayerSet::Layered(_, layer_groups) => {
                layer_groups.push(layer_group);
                creature.graphics_type.push(layer_set);
                creature_file.creatures.push(creature);
            },
            LayerSet::Simple(simple_layers) | LayerSet::Statue(simple_layers) => {
                simple_layers.push(simple_layer);
                creature.graphics_type.push(layer_set);
                creature_file.creatures.push(creature);
            }
        }

        Ok(creature_file)
    }

    fn rename_layer_groups(creature_files: &mut Vec<CreatureFile>) {
        for cf in creature_files.iter_mut() {
            for c in cf.creatures.iter_mut() {
                for gt in c.graphics_type.iter_mut() {
                    if let LayerSet::Layered(state, lgs) = gt {
                        for lg in lgs.iter_mut() {
                            if lg.name.eq("") {
                                let mut layer_names: Vec<String> = lg.layers.iter().map(|layer|layer.name.clone()).collect();
                                layer_names.sort();
                                layer_names.dedup();

                                match layer_names.len() {
                                    0 => lg.name = state.name().to_case(Case::Title),
                                    1 => lg.name = layer_names[0].clone(),
                                    _ => {
                                        let mut words: Vec<&str> = layer_names[0].split("_").collect();
                                        words.retain(|&elem| layer_names.iter().all(|n| n.contains(&elem)));

                                        if words.is_empty() {
                                            lg.name = state.name().to_case(Case::Title);
                                        } else {
                                            lg.name = words.join("_");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn export(&self, path: &path::PathBuf) -> Result<(), Box<dyn Error>> {
        fs::DirBuilder::new()
            .recursive(true)
            .create(path.join("graphics").join("images"))
            .expect("should always find or build directory in folder with permissions");

        for tile_page in self.tile_pages.iter() {
            tile_page.export(&path)?;
        }

        for creature_file in self.creature_files.iter() {
            creature_file.export(&path)?;
        }

        Ok(())
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

    fn export(&self, path: &path::PathBuf) -> Result<(), Box<dyn Error>> {
        let creature_file = fs::File::create(
            path
            .join("graphics")
            .join(format!("graphics_creatures_{}.txt",
            self.name.clone().to_case(Case::Snake)))
        ).expect("creature file creation should not fail");

        let mut creature_writer = io::LineWriter::new(creature_file);
        
        creature_writer.write_all(format!(
            "graphics_creatures_{}\n\n[OBJECT:GRAPHICS]\n\n",
            self.name.to_case(Case::Snake)
        ).as_bytes())?;

        for creature in self.creatures.iter() {
            creature_writer.write_all(creature.export()?.as_bytes()).expect("should always push a successful string");
        }
        
        creature_writer.flush().expect("flush should succeed for any reasonable case");

        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
enum State {
    #[default]
    Empty,
    Default,
    Child,
    Animated,
    Corpse,
    ListIcon,
    TrainedHunter,
    TrainedWar,
    Skeleton,
    SkeletonWithSkull,
    Custom(String),
}
impl State {
    fn name(&self) -> String {
        match self {
            Self::Default => "DEFAULT".to_string(),
            Self::Child => "CHILD".to_string(),
            Self::Animated => "ANIMATED".to_string(),
            Self::Corpse => "CORPSE".to_string(),
            Self::ListIcon => "LIST_ICON".to_string(),
            Self::TrainedHunter => "TRAINED_HUNTER".to_string(),
            Self::TrainedWar => "TRAINED_WAR".to_string(),
            Self::Skeleton => "SKELETON".to_string(),
            Self::SkeletonWithSkull => "SKELETON_WITH_SKULL".to_string(),
            Self::Custom(name) => name.to_string(),
            Self::Empty => "(empty)".to_string(),
        }
    }

    fn iterator() -> std::slice::Iter<'static, Self> {
        static STATES: [State; 9] = [
            State::Default,
            State::Child,
            State::Animated,
            State::Corpse,
            State::ListIcon,
            State::TrainedHunter,
            State::TrainedWar,
            State::Skeleton,
            State::SkeletonWithSkull,
        ];
        STATES.iter()
    }
}
impl From<String> for State {
    fn from(string: String) -> Self {
        match string.to_uppercase().as_str() {
            "DEFAULT" => {State::Default},
            "CHILD" => {State::Child},
            "ANIMATED" => State::Animated,
            "CORPSE" => State::Corpse,
            "LIST_ICON" => State::ListIcon,
            "TRAINED_HUNTER" => State::TrainedHunter,
            "TRAINED_WAR" => State::TrainedWar,
            "SKELETON" => State::Skeleton,
            "SKELETON_WITH_SKULL" => State::SkeletonWithSkull,
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
    // fn name(&self) -> String {
    //     match self {
    //         LayerSet::Simple(..) => "SIMPLE".to_string(),
    //         LayerSet::Statue(..) => "STATUE".to_string(),
    //         LayerSet::Layered(..) => "LAYERED".to_string(),
    //         LayerSet::Empty => "(none)".to_string(),
    //         _ => "(unexpected state)".to_string(),
    //     }
    // }

    fn layer_set_menu(&mut self, ui: &mut Ui) {
        ui.separator();

        match self {
            LayerSet::Layered(state, layer_groups) => {
                egui::ComboBox::from_label("State")
                    .selected_text(state.name())
                    .show_ui(ui, |ui| {
                    for s in State::iterator() {
                        ui.selectable_value(state, s.clone(), s.name());
                    }
                    ui.selectable_value(state, State::Custom(String::new()), "Custom");
                });
                if let State::Custom(s) = state {
                    ui.text_edit_singleline(s);
                }
                ui.label("Note: Although ANIMATED is used in vanilla, only DEFAULT and CORPSE appear to work properly (v50.05)");

                ui.add_space(PADDING);
                if ui.button("Add layer group").clicked() {
                    layer_groups.push(LayerGroup::new());
                }
            },
            LayerSet::Empty => {
                egui::ComboBox::from_label("Graphics Type")
                    .selected_text("(none)")
                    .show_ui(ui, |ui| {
                    ui.selectable_value(self, LayerSet::Layered(State::Default, Vec::new()), "Layered");
                    ui.selectable_value(self, LayerSet::Simple(vec![SimpleLayer::new()]), "Simple");
                    ui.selectable_value(self, LayerSet::Statue(vec![SimpleLayer::new()]), "Statue");
                });
            }
            _ => {},
        }
    }

    fn export(&self) -> Result<String, Box<dyn Error>> {
        let mut out = String::new();

        match self {
            LayerSet::Simple(simple_layers) => {
                for simple_layer in simple_layers {
                    out.push_str(&simple_layer.export()?);
                }

                out.push_str("\n");
            },
            LayerSet::Statue(simple_layers) => {
                for simple_layer in simple_layers {
                    out.push_str(&simple_layer.export_statue()?);
                }

                out.push_str("\n");
            },
            LayerSet::Layered(state, layer_groups) => {
                out.push_str(&format!("\t[LAYER_SET:{}]\n", state.name()));

                for layer_group in layer_groups {
                    out.push_str(&layer_group.export()?);
                }
            },
            LayerSet::Empty => {}
        }

        Ok(out)
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
            graphics_type: Vec::new(),
        }
    }

    fn empty() -> Creature {
        Creature {
            name: String::new(),
            graphics_type: Vec::new(),
        }
    }

    fn creature_menu(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.text_edit_singleline(&mut self.name);

        ui.add_space(PADDING);
        if ui.button("Add simple layer").clicked() {
            if self.graphics_type.iter().any(|ls| match ls { LayerSet::Layered(..) => true, _ => false, }) {
                self.graphics_type.insert(0, LayerSet::Simple(vec![SimpleLayer::empty()]));
            } else {
                self.graphics_type.push(LayerSet::Simple(vec![SimpleLayer::empty()]));
            }
        }
        if ui.button("Add layer set").clicked() {
            self.graphics_type.push(LayerSet::Layered(State::Default, Vec::new()));
        }
        if self.graphics_type.is_empty() {
            if ui.button("Add statue graphics (requires empty creature)").clicked() {
                self.graphics_type.push(LayerSet::Statue(vec![SimpleLayer::empty()]));
            }
        } else {
            ui.label("Statue graphics require an empty creature.");
        }
    }

    fn export(&self) -> Result<String, Box<dyn Error>> {
        let mut out = String::new();

        for graphics_type in self.graphics_type.iter() {
            if let LayerSet::Statue(_) = graphics_type {
                if out.is_empty() && (self.graphics_type.len() == 1) {
                    out.push_str(&format!(
                        "[STATUE_CREATURE_GRAPHICS:{}]\n",
                        self.name.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                    ));
                } else if self.graphics_type.len() > 1 {
                    panic!("statue graphics with too many layers");
                }
            } else if let LayerSet::Empty = graphics_type {
                
            } else {
                if out.is_empty() {
                    out.push_str(&format!(
                        "[CREATURE_GRAPHICS:{}]\n",
                        self.name.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                    ));
                }
            }
            
            out.push_str(&graphics_type.export()?);
        }

        Ok(out)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct SimpleLayer {
    state: State,
    tile: String,
    coords: [u32; 2],
    large_coords: Option<[u32; 2]>,
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

    fn new() -> SimpleLayer {
        SimpleLayer {
            state: State::Default,
            tile: String::new(),
            coords: [0, 0],
            large_coords: None,
            sub_state: None,
        }
    }

    fn layer_menu(&mut self, ui: &mut Ui, tile_info: Vec<(String, [u32;2])>) {
        let [x1, y1] = &mut self.coords;
        let state = &mut self.state;
        let sub_state = &mut self.sub_state;
        let (tile_names, max_coords) = DFGraphicsHelper::tile_read(&tile_info, &self.tile);
        
        egui::ComboBox::from_label("State")
            .selected_text(state.name())
            .show_ui(ui, |ui| {
            for s in State::iterator() {
                ui.selectable_value(state, s.clone(), s.name());
            }
            ui.selectable_value(state, State::Custom(String::new()), "Custom");
        });
        if let State::Custom(cust_state) = state {
            ui.label("Custom state:");
            ui.text_edit_singleline(cust_state);
            ui.hyperlink_to("Custom states that may work.", "https://dwarffortresswiki.org/index.php/Unit_type_token");
        }

        ui.add_space(PADDING);
        egui::ComboBox::from_label("Second state (optional)")
            .selected_text(sub_state.clone().unwrap_or(State::Empty).name())
            .show_ui(ui, |ui| {
            ui.selectable_value(sub_state, Some(State::Empty), State::Empty.name());
            for s in State::iterator() {
                ui.selectable_value(sub_state, Some(s.clone()), s.name());
            }
            ui.selectable_value(sub_state, Some(State::Custom(String::new())), "Custom");
        });
        if let Some(State::Custom(cust_state)) = sub_state {
            ui.label("Custom state:");
            ui.text_edit_singleline(cust_state);
            ui.hyperlink_to("Custom states that may work.", "https://dwarffortresswiki.org/index.php/Unit_type_token");
        }

        ui.add_space(PADDING);
        egui::ComboBox::from_label("Tile")
            .selected_text(&self.tile)
            .show_ui(ui, |ui| {
            for (t, _) in &tile_info {
                ui.selectable_value(&mut self.tile, t.clone(), t);
            }
            ui.selectable_value(&mut self.tile, String::new(), "Custom");
        });
        if !tile_names.contains(&self.tile) {
            ui.label("Custom tile name:");
            ui.text_edit_singleline(&mut self.tile);
        }

        ui.add_space(PADDING);
        let mut large = self.large_coords.is_some();
        ui.checkbox(&mut large, "Large Image:");

        if large {
            let [x2, y2] = self.large_coords.get_or_insert([0, 0]);
            ui.add(egui::Slider::new(x1, 0..=max_coords[0]-*x2).prefix("Tile X: "));
            ui.add(egui::Slider::new(y1, 0..=max_coords[1]-*y2).prefix("Tile Y: "));

            ui.add(egui::Slider::new(x2, 0..=2).prefix("X + "));
            ui.add(egui::Slider::new(y2, 0..=1).prefix("Y + "));
        } else {
            ui.add(egui::Slider::new(x1, 0..=max_coords[0]).prefix("Tile X: "));
            ui.add(egui::Slider::new(y1, 0..=max_coords[1]).prefix("Tile Y: "));

            if self.large_coords.is_some() {
                self.large_coords.take();
            }
        }

        ui.add_space(PADDING);
        ui.label("Preview:");
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.add(egui::Label::new(self.export().expect("should always make a valid layer")).wrap(false));
        });
    }

    fn statue_layer_menu(&mut self, ui: &mut Ui, tile_info: Vec<(String, [u32;2])>) {
        let [x1, y1] = &mut self.coords;
        let state = &mut self.state;
        let (tile_names, max_coords) = DFGraphicsHelper::tile_read(&tile_info, &self.tile);
        
        egui::ComboBox::from_label("State")
            .selected_text(state.name())
            .show_ui(ui, |ui| {
            for s in State::iterator() {
                ui.selectable_value(state,  s.clone(), s.name());
            }
            ui.selectable_value(state, State::Custom(String::new()), "Custom");
        });
        if let State::Custom(cust_state) = state {
            ui.label("Custom state:");
            ui.text_edit_singleline(cust_state);
            ui.hyperlink_to("Custom states that may work.", "https://dwarffortresswiki.org/index.php/Unit_type_token");
        }
        ui.label("Note: only DEFAULT is known to work v50.05");

        ui.add_space(PADDING);
        egui::ComboBox::from_label("Tile")
            .selected_text(&self.tile)
            .show_ui(ui, |ui| {
            for t in &tile_names {
                ui.selectable_value(&mut self.tile, t.clone(), t);
            }
            ui.selectable_value(&mut self.tile, String::new(), "Custom");
        });
        if !tile_names.contains(&self.tile) {
            ui.label("Custom tile name:");
            ui.text_edit_singleline(&mut self.tile);
        }

        ui.add_space(PADDING);
        let [x2, y2] = self.large_coords.get_or_insert([0, 0]);

        ui.add(egui::Slider::new(x1, 0..=max_coords[0]-*x2).prefix("Tile X: "));
        ui.add(egui::Slider::new(y1, 0..=max_coords[1]-*y2).prefix("Tile Y: "));

        ui.add(egui::Slider::new(x2, 0..=2).prefix("X + "));
        ui.add(egui::Slider::new(y2, 0..=1).prefix("Y + "));

        ui.add_space(PADDING);
        ui.label("Preview:");
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.add(egui::Label::new(self.export().expect("should always make a valid layer")).wrap(false));
        });
    }

    fn export(&self) -> Result<String, Box<dyn Error>> {
        if let Some([x2, y2]) = self.large_coords {
            if let Some(sub_state) = &self.sub_state {
                Ok(format!(
                    "\t\t[{}:{}:LARGE_IMAGE:{}:{}:{}:{}:AS_IS:{}]\n",
                    self.state.name(),
                    self.tile,
                    self.coords[0],
                    self.coords[1],
                    self.coords[0] + x2,
                    self.coords[0] + y2,
                    sub_state.name(),
                ))
            } else {
                Ok(format!(
                    "\t[{}:{}:LARGE_IMAGE:{}:{}:{}:{}:AS_IS]\n",
                    self.state.name(),
                    self.tile,
                    self.coords[0],
                    self.coords[1],
                    self.coords[0] + x2,
                    self.coords[0] + y2,
                ))
            }
        } else {
            if let Some(sub_state) = &self.sub_state {
                Ok(format!(
                    "\t\t[{}:{}:{}:{}:AS_IS:{}]\n",
                    self.state.name(),
                    self.tile,
                    self.coords[0],
                    self.coords[1],
                    sub_state.name(),
                ))
            } else {
                Ok(format!(
                    "\t[{}:{}:{}:{}:AS_IS]\n",
                    self.state.name(),
                    self.tile,
                    self.coords[0],
                    self.coords[1],
                ))
            }
        }
    }

    fn export_statue(&self) -> Result<String, Box<dyn Error>> {
        let [x2, y2] = self.large_coords.ok_or("export statue without large coordinates")?;

        Ok(format!(
            "\t[{}:{}:{}:{}:{}:{}]\n",
            self.state.name(),
            self.tile,
            self.coords[0],
            self.coords[1],
            self.coords[0] + x2,
            self.coords[1] + y2,
        ))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Layer {
    name: String,                     //LAYER_NAME for patterning
    conditions: Vec<Condition>,       //Set of condition(s) that layer displays in
    tile: String,                     //TILE_NAME of image
    coords: [u32; 2],               //x,y coordinates of layer on image in tiles
    large_coords: Option<[u32; 2]>, //(optional) x2,y2 coordinates of bottom right corner of layer in tiles
}
impl Layer {
    fn new() -> Layer {
        Layer {
            name: "(new)".to_string(),
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

    fn layer_menu(&mut self, ui: &mut Ui, tile_info: Vec<(String, [u32; 2])>) {
        
        let layer = self.clone();
        let [x1, y1] = &mut self.coords;
        let conditions = &mut self.conditions;
        let (tile_names, max_coords) = DFGraphicsHelper::tile_read(&tile_info, &self.tile);

        ui.separator();

        ui.columns(2, |ui| {
            ui[0].label("Layer name:");
            ui[0].text_edit_singleline(&mut self.name);
            ui[0].add_space(PADDING);
            
            egui::ComboBox::from_label("Tile:")
            .selected_text(&self.tile)
            .show_ui(&mut ui[0], |ui| {
                ui.selectable_value(&mut self.tile, String::from("(select)"), "(select)");
                for tile_name in tile_names {
                    ui.selectable_value(
                        &mut self.tile,
                        tile_name.to_string(),
                        tile_name,
                    );
                }
                ui.selectable_value(&mut self.tile, String::new(), "New Tile");
            });
            ui[0].text_edit_singleline(&mut self.tile);

            ui[0].add_space(PADDING);
            let mut large = self.large_coords.is_some();
            ui[0].checkbox(&mut large, "Large Image:");

            if large {
                let [x2, y2] = self.large_coords.get_or_insert([0, 0]);
                ui[0].add(egui::Slider::new(x1, 0..=max_coords[0]-*x2).prefix("Tile X: "));
                ui[0].add(egui::Slider::new(y1, 0..=max_coords[1]-*y2).prefix("Tile Y: "));

                ui[0].add(egui::Slider::new(x2, 0..=2).prefix("X + "));
                ui[0].add(egui::Slider::new(y2, 0..=1).prefix("Y + "));
            } else {
                ui[0].add(egui::Slider::new(x1, 0..=max_coords[0]).prefix("Tile X: "));
                ui[0].add(egui::Slider::new(y1, 0..=max_coords[1]).prefix("Tile Y: "));

                if self.large_coords.is_some() {
                    self.large_coords.take();
                }
            }

            ui[0].add_space(PADDING);
            if ui[0].button("Add Condition").clicked() {
                conditions.push(Condition::default());
            }

            ui[0].add_space(PADDING);
            ui[0].label("Preview:");
            egui::ScrollArea::horizontal()
                .id_source("Preview scroll")
                .show(&mut ui[0], |ui| {
                ui.add(egui::Label::new(
                    layer.export().expect("should always create valid string"))
                    .wrap(false)
                );
            });

            let mut delete = None;
            
            egui::ScrollArea::vertical()
                .id_source("Condition scroll")
                .max_height(300.0)
                .show(&mut ui[1], |ui| {
                for (i_cond, condition) in conditions.iter_mut().enumerate() {
                    ui.push_id(i_cond, |ui| {
                        condition.condition_menu(ui, tile_info.clone());
                        ui.add_space(PADDING);
                        if ui.button("Remove Condition").clicked() {
                            delete = Some(i_cond);
                        }
                        ui.add_space(PADDING);
                    });
                }
            });

            if let Some(i_cond) = delete {
                conditions.remove(i_cond);
            }
        });
    }

    fn export(&self) -> Result<String, Box<dyn Error>> {
        let mut out = String::new();

        if let Some([x2, y2]) = self.large_coords {
            out.push_str(&format!(
                "\t\t\t[LAYER:{}:{}:LARGE_IMAGE:{}:{}:{}:{}:AS_IS]\n",
                self.name,
                self.tile,
                self.coords[0],
                self.coords[1],
                self.coords[0] + x2,
                self.coords[1] + y2,
            ));
        } else {
            out.push_str(&format!(
                "\t\t\t[LAYER:{}:{}:{}:{}:AS_IS]\n",
                self.name,
                self.tile,
                self.coords[0],
                self.coords[1],
            ));
        }

        for condition in self.conditions.iter() {
            out.push_str(&condition.export()?);
        }

        Ok(out)
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
            name: "(new)".to_string(),
            layers: vec![Layer::new()],
        }
    }

    fn empty() -> LayerGroup {
        LayerGroup {
            name: String::new(),
            layers: Vec::new(),
        }
    }

    fn layer_group_menu(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.label("Layer group name:");
        ui.text_edit_singleline(&mut self.name);

        ui.add_space(PADDING);
        if ui.button("New Layer").clicked() {
            self.layers.push(Layer { name: self.name.clone(), ..Layer::new() });
        }

        ui.add_space(PADDING);
        ui.label("Preview:");
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.add(egui::Label::new(self.export().expect("should always make a valid layer")).wrap(false));
        });
    }

    fn export(&self) -> Result<String, Box<dyn Error>> {
        let mut out = String::new();

        // dbg!(&self.name);

        out.push_str(&format!("\t\t[LAYER_GROUP] ---{}---\n", self.name));

        for layer in self.layers.iter() {
            out.push_str(&layer.export()?);
        }

        out.push_str("\n");

        Ok(out)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
enum Metal {
    #[default]
    None,
    Copper,
    Silver,
    Bronze,
    BlackBronze,
    Iron,
    Steel,
    Adamantine,
    Custom(String),
}
impl Metal {
    fn name(&self) -> String {
        match self {
            Metal::None => String::new(),
            Metal::Copper => "COPPER".to_string(),
            Metal::Silver => "SILVER".to_string(),
            Metal::Bronze => "BRONZE".to_string(),
            Metal::BlackBronze => "BLACK_BRONZE".to_string(),
            Metal::Iron => "IRON".to_string(),
            Metal::Steel => "STEEL".to_string(),
            Metal::Adamantine => "ADAMANTINE".to_string(),
            Metal::Custom(metal) => metal.clone(),
        }
    }

    fn from(string: String) -> Metal {
        match string.as_str() {
            "COPPER" => Metal::Copper,
            "SILVER" => Metal::Silver,
            "BRONZE" => Metal::Bronze,
            "BLACK_BRONZE" => Metal::BlackBronze,
            "IRON" => Metal::Iron,
            "STEEL" => Metal::Steel,
            "ADAMANTINE" => Metal::Adamantine,
            metal => Metal::Custom(metal.to_string()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
enum MaterialFlag {
    #[default]
    None,
    DivineMaterial,
    Artifact,
    NotArtifact,
    Leather,
    Bone,
    Shell,
    Wood,
    Woven,
    Silk,
    Yarn,
    Plant,
    NotImproved,
    Empty,
    Stone,
    Gem,
    Tooth,
    Horn,
    Pearl,
    Soap,
    HardMat,
    Metal,
    Glass,
    Sand,
    StrandTissue,
    Lye,
    Potashable,
    FoodStorage,
    EmptyBarrel,
    NotPressed,
    FireSafe,
    MagmaSafe,
    BuildMat,
    WorthlessStone,
    BodyComp,
    CanUseLocation,
    NoEdge,
    Edge,
    NotEngraved,
    WritingImprovment,
    NotAbsorb,
    Unrotten,
    NotWeb,
    Web,
    CanArtifact,
    OnGround,
}
impl MaterialFlag {
    fn name(&self) -> String {
        match self {
            MaterialFlag::None => String::new(),
            MaterialFlag::DivineMaterial => "IS_DIVINE_MATERIAL".to_string(),
            MaterialFlag::Artifact => "IS_CRAFTED_ARTIFACT".to_string(),
            MaterialFlag::NotArtifact => "NOT_ARTIFACT".to_string(),
            MaterialFlag::Leather => "ANY_LEATHER_MATERIAL".to_string(),
            MaterialFlag::Bone => "ANY_BONE_MATERIAL".to_string(),
            MaterialFlag::Shell => "ANY_SHELL_MATERIAL".to_string(),
            MaterialFlag::Wood => "ANY_WOOD_MATERIAL".to_string(),
            MaterialFlag::Woven => "WOVEN_ITEM".to_string(),
            MaterialFlag::Silk => "ANY_SILK_MATERIAL".to_string(),
            MaterialFlag::Yarn => "ANY_YARN_MATERIAL".to_string(),
            MaterialFlag::Plant => "ANY_PLANT_MATERIAL".to_string(),
            MaterialFlag::NotImproved => "NOT_IMPROVED".to_string(),
            MaterialFlag::Empty => "EMPTY".to_string(),
            MaterialFlag::StrandTissue => "ANY_STRAND_TISSUE".to_string(),
            MaterialFlag::Stone => "ANY_STONE_MATERIAL".to_string(),
            MaterialFlag::Gem => "ANY_GEM_MATERIAL".to_string(),
            MaterialFlag::Tooth => "ANY_TOOTH_MATERIAL".to_string(),
            MaterialFlag::Horn => "ANY_HORN_MATERIAL".to_string(),
            MaterialFlag::Pearl => "ANY_PEARL_MATERIAL".to_string(),
            MaterialFlag::Soap => "ANY_SOAP_MATERIAL".to_string(),
            MaterialFlag::HardMat => "HARD_ITEM_MATERIAL".to_string(),
            MaterialFlag::Metal => "METAL_ITEM_MATERIAL".to_string(),
            MaterialFlag::Glass => "GLASS_MATERIAL".to_string(),
            MaterialFlag::Sand => "IS_SAND_MATERIAL".to_string(),
            MaterialFlag::Lye => "CONTAINS_LYE".to_string(),
            MaterialFlag::Potashable => "POTASHABLE".to_string(),
            MaterialFlag::FoodStorage => "FOOD_STORAGE_CONTAINER".to_string(),
            MaterialFlag::EmptyBarrel => "NOT_CONTAIN_BARREL_ITEM".to_string(),
            MaterialFlag::NotPressed => "NOT_PRESSED".to_string(),
            MaterialFlag::FireSafe => "FIRE_BUILD_SAFE".to_string(),
            MaterialFlag::MagmaSafe => "MAGMA_BUILD_SAFE".to_string(),
            MaterialFlag::BuildMat => "BUILDMAT".to_string(),
            MaterialFlag::WorthlessStone => "WORTHLESS_STONE_ONLY".to_string(),
            MaterialFlag::BodyComp => "USE_BODY_COMPONENT".to_string(),
            MaterialFlag::CanUseLocation => "CAN_USE_LOCATION_RESERVED".to_string(),
            MaterialFlag::NoEdge => "NO_EDGE_ALLOWED".to_string(),
            MaterialFlag::Edge => "HAS_EDGE".to_string(),
            MaterialFlag::NotEngraved => "NOT_ENGRAVED".to_string(),
            MaterialFlag::WritingImprovment => "HAS_WRITING_IMPROVEMENT".to_string(),
            MaterialFlag::NotAbsorb => "DOES_NOT_ABSORB".to_string(),
            MaterialFlag::Unrotten => "UNROTTEN".to_string(),
            MaterialFlag::NotWeb => "NOT_WEB".to_string(),
            MaterialFlag::Web => "WEB_ONLY".to_string(),
            MaterialFlag::CanArtifact => "CAN_USE_ARTIFACT".to_string(),
            MaterialFlag::OnGround => "ON_GROUND".to_string(),
        }
    }

    fn from(string: String) -> MaterialFlag {
        match string.as_str() {
            "IS_DIVINE_MATERIAL" => MaterialFlag::DivineMaterial,
            "IS_CRAFTED_ARTIFACT" => MaterialFlag::Artifact,
            "NOT_ARTIFACT" => MaterialFlag::NotArtifact,
            "ANY_LEATHER_MATERIAL" => MaterialFlag::Leather,
            "ANY_BONE_MATERIAL" => MaterialFlag::Bone,
            "ANY_SHELL_MATERIAL" => MaterialFlag::Shell,
            "ANY_WOOD_MATERIAL" => MaterialFlag::Wood,
            "WOVEN_ITEM" => MaterialFlag::Woven,
            "ANY_SILK_MATERIAL" => MaterialFlag::Silk,
            "ANY_YARN_MATERIAL" => MaterialFlag::Yarn,
            "ANY_PLANT_MATERIAL" => MaterialFlag::Plant,
            "NOT_IMPROVED" => MaterialFlag::NotImproved,
            "EMPTY" => MaterialFlag::Empty,
            "ANY_STRAND_TISSUE" => MaterialFlag::StrandTissue,
            "ANY_STONE_MATERIAL" => MaterialFlag::Stone,
            "ANY_GEM_MATERIAL" => MaterialFlag::Gem,
            "ANY_TOOTH_MATERIAL" => MaterialFlag::Tooth,
            "ANY_HORN_MATERIAL" => MaterialFlag::Horn,
            "ANY_PEARL_MATERIAL" => MaterialFlag::Pearl,
            "ANY_SOAP_MATERIAL" => MaterialFlag::Soap,
            "HARD_ITEM_MATERIAL" => MaterialFlag::HardMat,
            "METAL_ITEM_MATERIAL" => MaterialFlag::Metal,
            "GLASS_MATERIAL" => MaterialFlag::Glass,
            "IS_SAND_MATERIAL" => MaterialFlag::Sand,
            "CONTAINS_LYE" => MaterialFlag::Lye,
            "POTASHABLE" => MaterialFlag::Potashable,
            "FOOD_STORAGE_CONTAINER" => MaterialFlag::FoodStorage,
            "NOT_CONTAIN_BARREL_ITEM" => MaterialFlag::EmptyBarrel,
            "NOT_PRESSED" => MaterialFlag::NotPressed,
            "FIRE_BUILD_SAFE" => MaterialFlag::FireSafe,
            "MAGMA_BUILD_SAFE" => MaterialFlag::MagmaSafe,
            "BUILDMAT" => MaterialFlag::BuildMat,
            "WORTHLESS_STONE_ONLY" => MaterialFlag::WorthlessStone,
            "USE_BODY_COMPONENT" => MaterialFlag::BodyComp,
            "CAN_USE_LOCATION_RESERVED" => MaterialFlag::CanUseLocation,
            "NO_EDGE_ALLOWED" => MaterialFlag::NoEdge,
            "HAS_EDGE" => MaterialFlag::Edge,
            "NOT_ENGRAVED" => MaterialFlag::NotEngraved,
            "HAS_WRITING_IMPROVEMENT" => MaterialFlag::WritingImprovment,
            "DOES_NOT_ABSORB" => MaterialFlag::NotAbsorb,
            "UNROTTEN" => MaterialFlag::Unrotten,
            "NOT_WEB" => MaterialFlag::NotWeb,
            "WEB_ONLY" => MaterialFlag::Web,
            "CAN_USE_ARTIFACT" => MaterialFlag::CanArtifact,
            "ON_GROUND" => MaterialFlag::OnGround,
            _ => MaterialFlag::None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
enum ItemType {
    #[default]
    None,
    ByCategory(String, Equipment),
    ByToken(String, Equipment),
    AnyHeld(Equipment),
    Wield(Equipment),
}
impl ItemType {
    fn name(&self) -> String {
        match self {
            ItemType::None => String::new(),
            ItemType::ByCategory(..) => "BY_CATEGORY".to_string(),
            ItemType::ByToken(..) => "BY_TOKEN".to_string(),
            ItemType::AnyHeld(..) => "ANY_HELD".to_string(),
            ItemType::Wield(..) => "WIELD".to_string(),
        }
    }

    fn from(strings: Vec<String>) -> (ItemType, Vec<String>) {
        match strings[0].as_str() {
            "BY_CATEGORY" => {
                (ItemType::ByCategory(strings[1].clone(),
                Equipment::from(strings[2].clone())),
                strings[3..].to_vec())
            },
            "BY_TOKEN" => {
                (ItemType::ByToken(strings[1].clone(),
                Equipment::from(strings[2].clone())),
                strings[3..].to_vec())
            },
            "ANY_HELD" => {
                (ItemType::AnyHeld(Equipment::from(strings[1].clone())),
                strings[2..].to_vec())
            },
            "WIELD" => {
                (ItemType::Wield(Equipment::from(strings[1].clone())),
                strings[2..].to_vec())
            },
            _ => {(ItemType::None, strings)}
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
enum Equipment {
    #[default]
    None,
    Armor,
    Helm,
    Gloves,
    Shoes,
    Pants,
    Shield,
    Weapon,
    Any,
}
impl Equipment {
    fn name(&self) -> String {
        match self {
            Equipment::None => String::new(),
            Equipment::Armor => "ARMOR".to_string(),
            Equipment::Helm => "HELM".to_string(),
            Equipment::Gloves => "GLOVES".to_string(),
            Equipment::Shoes => "SHOES".to_string(),
            Equipment::Pants => "PANTS".to_string(),
            Equipment::Shield => "SHIELD".to_string(),
            Equipment::Weapon => "WEAPON".to_string(),
            Equipment::Any => "ANY".to_string(),
        }
    }

    fn from(string: String) -> Equipment {
        match string.as_str() {
            "ARMOR" => Equipment::Armor,
            "HELM" => Equipment::Helm,
            "GLOVES" => Equipment::Gloves,
            "SHOES" => Equipment::Shoes,
            "PANTS" => Equipment::Pants,
            "SHIELD" => Equipment::Shield,
            "WEAPON" => Equipment::Weapon,
            "ANY" => Equipment::Any,
            _ => Equipment::None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
enum Profession {
    #[default]
    Empty,
    Stoneworker,
    Miner,
    Metalsmith,
    Engineer,
    Farmer,
    Woodworker,
    Jeweler,
    Ranger,
    Standard,
    Craftsman,
    FisheryWorker,
    Merchant,
    Child,
    None,
    Custom(String),
}
impl Profession {
    fn name(&self) -> String {
        match self {
            Profession::Empty => String::new(),
            Profession::Stoneworker => "STONEWORKER".to_string(),
            Profession::Miner => "MINER".to_string(),
            Profession::Metalsmith => "METALSMITH".to_string(),
            Profession::Engineer => "ENGINEER".to_string(),
            Profession::Farmer => "FARMER".to_string(),
            Profession::Woodworker => "WOODWORKER".to_string(),
            Profession::Jeweler => "JEWELER".to_string(),
            Profession::Ranger => "RANGER".to_string(),
            Profession::Standard => "STANDARD".to_string(),
            Profession::Craftsman => "CRAFTSMAN".to_string(),
            Profession::FisheryWorker => "FISHERY_WORKER".to_string(),
            Profession::Merchant => "MERCHANT".to_string(),
            Profession::Child => "CHILD".to_string(),
            Profession::None => "NONE".to_string(),
            Profession::Custom(prof) => prof.clone(),
        }
    }

    fn from(string: String) -> Profession {
        match string.as_str() {
            "STONEWORKER" => Profession::Stoneworker,
            "MINER" => Profession::Miner,
            "METALSMITH" => Profession::Metalsmith,
            "ENGINEER" => Profession::Engineer,
            "FARMER" => Profession::Farmer,
            "WOODWORKER" => Profession::Woodworker,
            "JEWELER" => Profession::Jeweler,
            "RANGER" => Profession::Ranger,
            "STANDARD" => Profession::Standard,
            "CRAFTSMAN" => Profession::Craftsman,
            "FISHERY_WORKER" => Profession::Ranger,
            "MERCHANT" => Profession::Ranger,
            "NONE" => Profession::Ranger,
            prof=> Profession::Custom(prof.to_string()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
enum Condition {
    #[default]
    Default,
    ItemWorn(ItemType, Vec<String>),
    ShutOffIfItemPresent(ItemType, Vec<String>),
    Dye(String),
    NotDyed,
    MaterialFlag(Vec<MaterialFlag>),
    MaterialType(Metal),
    ProfessionCategory(Vec<Profession>),
    RandomPartIndex(String, u32, u32),
    HaulCountMin(u32),
    HaulCountMax(u32),
    Child,
    NotChild,
    Caste(String),
    Ghost,
    SynClass(Vec<String>),
    TissueLayer(String, String),
    TissueMinLength(u32),
    TissueMaxLength(u32),
    TissueMayHaveColor(Vec<String>),
    TissueMayHaveShaping(Vec<String>),
    TissueNotShaped,
    TissueSwap(String, u32, String, [u32;2], Option<[u32;2]>),
    Custom(String),
}
impl Condition {
    fn name(&self) -> String {
        match self {
            Condition::Default => "(default)".to_string(),
            Condition::ItemWorn(..) => "CONDITION_ITEM_WORN".to_string(),
            Condition::ShutOffIfItemPresent(..) => "SHUT_OFF_IF_ITEM_PRESENT".to_string(),
            Condition::Dye(..) => "CONDITION_DYE".to_string(),
            Condition::NotDyed => "CONDITION_NOT_DYED".to_string(),
            Condition::MaterialFlag(..) => "CONDITION_MATERIAL_FLAG".to_string(),
            Condition::MaterialType(..) => "CONDITION_MATERIAL_TYPE".to_string(),
            Condition::ProfessionCategory(..) => "CONDITION_PROFESSION_CATEGORY".to_string(),
            Condition::RandomPartIndex(..) => "CONDITION_RANDOM_PART_INDEX".to_string(),
            Condition::HaulCountMin(..) => "CONDITION_HAUL_COUNT_MIN".to_string(),
            Condition::HaulCountMax(..) => "CONDITION_HAUL_COUNT_MAX".to_string(),
            Condition::Child => "CONDITION_CHILD".to_string(),
            Condition::NotChild => "CONDITION_NOT_CHILD".to_string(),
            Condition::Caste(..) => "CONDITION_CASTE".to_string(),
            Condition::Ghost => "CONDITION_GHOST".to_string(),
            Condition::SynClass(..) => "CONDITION_SYN_CLASS".to_string(),
            Condition::TissueLayer(..) => "CONDITION_TISSUE_LAYER".to_string(),
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
            "CONDITION_ITEM_WORN" => {
                let (item_type, items) = ItemType::from(line_vec[1..].to_vec());
                Condition::ItemWorn(item_type, items)
            },
            "SHUT_OFF_IF_ITEM_PRESENT" => {
                let (item_type, items) = ItemType::from(line_vec[1..].to_vec());
                Condition::ShutOffIfItemPresent(item_type, items)
            },
            "CONDITION_DYE" => Condition::Dye(
                line_vec[1].clone()
            ),
            "CONDITION_NOT_DYED" => Condition::NotDyed,
            "CONDITION_MATERIAL_FLAG" => Condition::MaterialFlag(
                line_vec[1..]
                    .iter()
                    .map(|flag| MaterialFlag::from(flag.clone()))
                    .collect()
            ),
            "CONDITION_MATERIAL_TYPE" => Condition::MaterialType(
                Metal::from(line_vec[2].clone())
            ),
            "CONDITION_PROFESSION_CATEGORY" => Condition::ProfessionCategory(
                line_vec[1..]
                    .iter()
                    .map(|prof| Profession::from(prof.clone()))
                    .collect()
            ),
            "CONDITION_RANDOM_PART_INDEX" => Condition::RandomPartIndex(
                line_vec[1].clone(),
                line_vec[2].parse::<u32>().unwrap_or_default(),
                line_vec[3].parse::<u32>().unwrap_or_default()
            ),
            "CONDITION_HAUL_COUNT_MIN" => Condition::HaulCountMin(
                line_vec[1].parse::<u32>().unwrap_or_default()
            ),
            "CONDITION_HAUL_COUNT_MAX" => Condition::HaulCountMax(
                line_vec[1].parse::<u32>().unwrap_or_default()
            ),
            "CONDITION_CHILD" => Condition::Child,
            "CONDITION_NOT_CHILD" => Condition::NotChild,
            "CONDITION_CASTE" => Condition::Caste(
                line_vec[1].clone()
            ),
            "CONDITION_GHOST" => Condition::Ghost,
            "CONDITION_SYN_CLASS" => Condition::SynClass(
                line_vec.drain(1..).collect()
            ),
            "CONDITION_TISSUE_LAYER" => Condition::TissueLayer(
                line_vec[2].clone(),
                line_vec[3].clone(),
            ),
            "TISSUE_MIN_LENGTH" => Condition::TissueMinLength(
                line_vec[1].parse::<u32>().unwrap_or_default()
            ),
            "TISSUE_MAX_LENGTH" => Condition::TissueMaxLength(
                line_vec[1].parse::<u32>().unwrap_or_default()
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
                    let c = [line_vec[5].parse::<u32>().unwrap_or_default(),
                        line_vec[6].parse::<u32>().unwrap_or_default()];
                    let l_c = [line_vec[7].parse::<u32>().unwrap_or_default(),
                        line_vec[8].parse::<u32>().unwrap_or_default()];
                    let large;
                    if (c[0] <= l_c[0]) && (c[1] <= l_c[1]) {
                        large = [l_c[0]-c[0], l_c[1]-c[1]];
                    } else {
                        panic!("impossible large coordinates import condition");
                    }
                    Condition::TissueSwap(
                        line_vec[1].clone(),
                        line_vec[2].parse::<u32>().unwrap_or_default(),
                        line_vec[3].clone(),
                        c,
                        Some(large),
                    )
                } else {
                    Condition::TissueSwap(
                        line_vec[1].clone(),
                        line_vec[2].parse::<u32>().unwrap_or_default(),
                        line_vec[3].clone(),
                        [line_vec[4].parse::<u32>().unwrap_or_default(), 
                        line_vec[5].parse::<u32>().unwrap_or_default()],
                        None,
                    )
                }
            },
            other => Condition::Custom(other.to_string()),
        }
    }

    fn condition_menu(&mut self, ui: &mut Ui, tile_info: Vec<(String, [u32; 2])>) {
        egui::ComboBox::from_label("Condition type")
            .selected_text(&self.name())
            .show_ui(ui, |ui| {
                ui.selectable_value(self, Condition::Default, "(select)");
                ui.selectable_value(
                    self,
                    Condition::ItemWorn(ItemType::None, Vec::new()),
                    "CONDITION_ITEM_WORN"
                );
                ui.selectable_value(
                    self,
                    Condition::ShutOffIfItemPresent(ItemType::None, Vec::new()),
                    "SHUT_OFF_IF_ITEM_PRESENT",
                );
                ui.selectable_value(self, Condition::Dye(String::new()), "CONDITION_DYE");
                ui.selectable_value(self, Condition::NotDyed, "CONDITION_NOT_DYED");
                ui.selectable_value(
                    self,
                    Condition::MaterialFlag(Vec::new()),
                    "CONDITION_MATERIAL_FLAG"
                );
                ui.selectable_value(
                    self,
                    Condition::MaterialType(Metal::default()),
                    "CONDITION_MATERIAL_TYPE"
                );
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
                    Condition::SynClass(Vec::new()),
                    "CONDITION_SYN_CLASS",
                );
                ui.selectable_value(
                    self,
                    Condition::TissueLayer(String::new(), String::new()),
                    "CONDITION_TISSUE_LAYER"
                );
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
            Condition::ItemWorn(item_type, items)
            | Condition::ShutOffIfItemPresent(item_type, items)=> {
                egui::ComboBox::from_label("Selection type")
                    .selected_text(&item_type.name())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(item_type, ItemType::None, "(none)");
                        ui.selectable_value(item_type, ItemType::ByCategory(String::new(), Equipment::default()), "By Category");
                        ui.selectable_value(item_type, ItemType::ByToken(String::new(), Equipment::default()), "By Token");
                        ui.selectable_value(item_type, ItemType::AnyHeld(Equipment::default()), "Any Held");
                        ui.selectable_value(item_type, ItemType::Wield(Equipment::default()), "Wield");
                });

                ui.label("Selection subtype:");
                match item_type {
                    ItemType::None => {},
                    ItemType::ByCategory(category, equipment) => {
                        ui.label("Category: (e.g. HEAD)");
                        ui.text_edit_singleline(category);

                        ui.label("Item type: (e.g. HELM)");
                        egui::ComboBox::from_label("Item type")
                            .selected_text(&equipment.name())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(equipment, Equipment::Armor, "Armor");
                                ui.selectable_value(equipment, Equipment::Helm, "Helm");
                                ui.selectable_value(equipment, Equipment::Gloves, "Gloves");
                                ui.selectable_value(equipment, Equipment::Shoes, "Shoes");
                                ui.selectable_value(equipment, Equipment::Pants, "Pants");
                                ui.selectable_value(equipment, Equipment::Shield, "Shield");
                                ui.selectable_value(equipment, Equipment::Weapon, "Weapon");
                        });

                        ui.label("Item: (e.g. ITEM_HELM_HELM)");

                        for item in &mut *items {
                            ui.text_edit_singleline(item);
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Add item").clicked() {
                                items.push("".into());
                            }
                            if ui.button("Remove item").clicked() && items.len() > 1
                            {
                                items.pop();
                            }
                        });
                        ui.add_space(PADDING);
                    }
                    ItemType::ByToken(token, equipment) => {
                        ui.label("Token: (e.g. RH for right hand)");
                        ui.text_edit_singleline(token);

                        ui.label("Item type: (e.g. GLOVES)");
                        egui::ComboBox::from_label("Item type")
                            .selected_text(&equipment.name())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(equipment, Equipment::Armor, "Armor");
                                ui.selectable_value(equipment, Equipment::Helm, "Helm");
                                ui.selectable_value(equipment, Equipment::Gloves, "Gloves");
                                ui.selectable_value(equipment, Equipment::Shoes, "Shoes");
                                ui.selectable_value(equipment, Equipment::Pants, "Pants");
                                ui.selectable_value(equipment, Equipment::Shield, "Shield");
                                ui.selectable_value(equipment, Equipment::Weapon, "Weapon");
                        });

                        ui.label("Item: (e.g. ITEM_GLOVES_GAUNTLETS)");

                        for item in &mut *items {
                            ui.text_edit_singleline(item);
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Add item").clicked() {
                                items.push("".into());
                            }
                            if ui.button("Remove item").clicked() && items.len() > 1
                            {
                                items.pop();
                            }
                        });
                        ui.add_space(PADDING);
                    }
                    ItemType::AnyHeld(equipment) => {
                        ui.label("Item type: (e.g. SHIELD)");
                        egui::ComboBox::from_label("Item type")
                            .selected_text(&equipment.name())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(equipment, Equipment::Armor, "Armor");
                                ui.selectable_value(equipment, Equipment::Helm, "Helm");
                                ui.selectable_value(equipment, Equipment::Gloves, "Gloves");
                                ui.selectable_value(equipment, Equipment::Shoes, "Shoes");
                                ui.selectable_value(equipment, Equipment::Pants, "Pants");
                                ui.selectable_value(equipment, Equipment::Shield, "Shield");
                                ui.selectable_value(equipment, Equipment::Weapon, "Weapon");
                        });

                        ui.label("Item: (e.g. ITEM_SHIELD_SHIELD)");

                        for item in &mut *items {
                            ui.text_edit_singleline(item);
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Add item").clicked() {
                                items.push("".into());
                            }
                            if ui.button("Remove item").clicked() && items.len() > 1
                            {
                                items.pop();
                            }
                        });
                        ui.add_space(PADDING);
                    }
                    ItemType::Wield(equipment) => {
                        ui.label("Item type: (WEAPON or ANY)");
                        egui::ComboBox::from_label("Item type")
                            .selected_text(&equipment.name())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(equipment, Equipment::Any, "Any");
                                ui.selectable_value(equipment, Equipment::Weapon, "Weapon");
                        });

                        if equipment == &Equipment::Any {
                            items.clear();
                        } else {
                            ui.label("Item: (e.g. ITEM_WEAPON_PICK)");
    
                            for item in &mut *items {
                                ui.text_edit_singleline(item);
                            }
    
                            ui.horizontal(|ui| {
                                if ui.button("Add item").clicked() {
                                    items.push("".into());
                                }
                                if ui.button("Remove item").clicked() && items.len() > 1
                                {
                                    items.pop();
                                }
                            });
                        }
                        ui.add_space(PADDING);
                    }
                }
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
            Condition::MaterialFlag(flags) => {
                for (i_flag, flag) in flags.iter_mut().enumerate() {
                    ui.push_id(format!("{}{}", flag.name(), i_flag), |ui| {
                        egui::ComboBox::from_label(
                            "Common material flags",
                        )
                        .selected_text(flag.name())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(flag, MaterialFlag::default(), "(select)");
                            ui.selectable_value(flag, MaterialFlag::DivineMaterial, "DIVINE_MATERIAL");
                            ui.selectable_value(flag, MaterialFlag::Artifact, "ARTIFACT");
                            ui.selectable_value(flag, MaterialFlag::NotArtifact, "NOT_ARTIFACT");
                            ui.selectable_value(flag, MaterialFlag::Leather, "ANY_LEATHER_MATERIAL");
                            ui.selectable_value(flag, MaterialFlag::Bone, "ANY_BONE_MATERIAL");
                            ui.selectable_value(flag, MaterialFlag::Shell, "ANY_SHELL_MATERIAL");
                            ui.selectable_value(flag, MaterialFlag::Wood, "ANY_WOOD_MATERIAL");
                            ui.selectable_value(flag, MaterialFlag::Woven, "WOVEN_ITEM");
                            ui.selectable_value(flag, MaterialFlag::Silk, "ANY_SILK_MATERIAL");
                            ui.selectable_value(flag, MaterialFlag::Yarn, "ANY_YARN_MATERIAL");
                            ui.selectable_value(flag, MaterialFlag::Plant, "ANY_PLANT_MATERIAL");
                            ui.selectable_value(flag, MaterialFlag::NotImproved, "NOT_IMPROVED");
                            ui.selectable_value(flag, MaterialFlag::Empty, "EMPTY");
                        });
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("Add flag").clicked() {
                        flags.push(MaterialFlag::default());
                    }
                    if ui.button("Remove flag").clicked() && flags.len() > 1 {
                        flags.pop();
                    }
                });

                ui.add_space(PADDING);
                ui.hyperlink_to("List of other useful flags.", "https://dwarffortresswiki.org/index.php/Graphics_token#CONDITION_MATERIAL_FLAG");
                ui.hyperlink_to("Full list of all possible flags (v50.05).", "http://www.bay12forums.com/smf/index.php?topic=169696.msg8442543#msg8442543");
            }
            Condition::MaterialType(metal) => {
                ui.label("Metals are the only material type known to work v50.05");
                egui::ComboBox::from_label(
                    "Material name:   (dropdown contains vanilla weapon metals)",
                )
                .selected_text(metal.name())
                .show_ui(ui, |ui| {
                    ui.selectable_value(metal, Metal::None, "(select)");
                    ui.selectable_value(metal, Metal::Copper, "COPPER");
                    ui.selectable_value(metal, Metal::Silver, "SILVER");
                    ui.selectable_value(metal, Metal::Bronze, "BRONZE");
                    ui.selectable_value(metal, Metal::BlackBronze, "BLACK_BRONZE");
                    ui.selectable_value(metal, Metal::Iron, "IRON");
                    ui.selectable_value(metal, Metal::Steel, "STEEL");
                    ui.selectable_value(metal, Metal::Adamantine, "ADAMANTINE");
                });

                ui.add_space(PADDING);
                ui.label(
                    "In vanilla, only used for metal weapons and armor (e.g. METAL:IRON).",
                );
                ui.hyperlink_to("At least some material flags are currently unusable (v50.05 //todo recheck).", "https://dwarffortresswiki.org/index.php/Graphics_token#CONDITION_MATERIAL_TYPE");
            }
            Condition::ProfessionCategory(professions) => {
                for profession in professions.iter_mut() {
                    ui.push_id(profession.name(), |ui| {
                        egui::ComboBox::from_label("Profession:   (dropdown contains known working ones)")
                            .selected_text(profession.name())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(profession, Profession::Empty, "(select)");
                                ui.selectable_value(profession, Profession::Stoneworker, Profession::Stoneworker.name());
                                ui.selectable_value(profession, Profession::Miner, Profession::Miner.name());
                                ui.selectable_value(profession, Profession::Metalsmith, Profession::Metalsmith.name());
                                ui.selectable_value(profession, Profession::Engineer, Profession::Engineer.name());
                                ui.selectable_value(profession, Profession::Farmer, Profession::Farmer.name());
                                ui.selectable_value(profession, Profession::Woodworker, Profession::Woodworker.name());
                                ui.selectable_value(profession, Profession::Jeweler, Profession::Jeweler.name());
                                ui.selectable_value(profession, Profession::Ranger, Profession::Ranger.name());
                                ui.selectable_value(profession, Profession::Standard, Profession::Standard.name());
                                ui.selectable_value(profession, Profession::Craftsman, Profession::Craftsman.name());
                                ui.selectable_value(profession, Profession::FisheryWorker, Profession::FisheryWorker.name());
                                ui.selectable_value(profession, Profession::Merchant, Profession::Merchant.name());
                                ui.selectable_value(profession, Profession::Child, Profession::Child.name());
                                ui.selectable_value(profession, Profession::None, Profession::None.name());
                                ui.selectable_value(profession, Profession::Custom(String::new()), "Custom");
                        });
                        if let Profession::Custom(prof) = profession {
                            ui.text_edit_singleline(prof);
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("Add profession").clicked() {
                        professions.push(Profession::Empty);
                    }
                    if ui.button("Remove profession").clicked() && professions.len() > 1 {
                        professions.pop();
                    }
                });

                ui.add_space(PADDING);
                ui.hyperlink_to(
                    "Full list of possible professions.",
                    "https://dwarffortresswiki.org/index.php/Unit_type_token#Profession_Categories",
                );
            }
            Condition::RandomPartIndex(id, index, max) => {
                ui.label("Random part identifier: (e.g. HEAD):");
                ui.text_edit_singleline(id);

                ui.add(
                    egui::DragValue::new(max)
                        .speed(1)
                        .prefix("Total parts: ")
                        .clamp_range(1..=u32::MAX),
                );

                ui.add(
                    egui::DragValue::new(index)
                        .speed(1)
                        .prefix("Part index: ")
                        .clamp_range(1..=*max),
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
            Condition::SynClass(syn_classes) => {
                ui.hyperlink_to(
                    "Syndrome class:",
                    "https://dwarffortresswiki.org/index.php/Graphics_token#CONDITION_SYN_CLASS",
                );
                for syn_class in syn_classes.iter_mut() {
                    ui.text_edit_singleline(syn_class);
                }
                ui.horizontal(|ui| {
                    if ui.button("Add syn_class").clicked() {
                        syn_classes.push(String::new());
                    }
                    if ui.button("Remove syn_class").clicked() && syn_classes.len() > 1 {
                        syn_classes.pop();
                    }
                });
            }
            Condition::TissueLayer(category, tissue) => {
                ui.label("BY_CATEGORY assumed because it is the only selection type allowed in v50.05");
                ui.label("Category: (e.g. HEAD or ALL)");
                ui.text_edit_singleline(category);
                ui.label("Tissue: (e.g. HAIR or ALL)");
                ui.text_edit_singleline(tissue);
                
                ui.add_space(PADDING);
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
                for color in colors.iter_mut() {
                    ui.text_edit_singleline(color);
                }
                ui.horizontal(|ui| {
                    if ui.button("Add color").clicked() {
                        colors.push(String::new());
                    }
                    if ui.button("Remove color").clicked() && colors.len() > 1 {
                        colors.pop();
                    }
                });

                ui.add_space(PADDING);
                ui.label("requires a CONDITION_TISSUE_LAYER above.");
            }
            Condition::TissueMayHaveShaping(shapings) => {
                ui.hyperlink_to(
                    "Shaping: (e.g. NEATLY_COMBED, PONY_TAILS, CLEAN_SHAVEN)",
                    "https://dwarffortresswiki.org/index.php/Entity_token#TS_PREFERRED_SHAPING",
                );
                for shaping in shapings.iter_mut() {
                    ui.text_edit_singleline(shaping);
                }
                ui.horizontal(|ui| {
                    if ui.button("Add color").clicked() {
                        shapings.push(String::new());
                    }
                    if ui.button("Remove color").clicked() && shapings.len() > 1 {
                        shapings.pop();
                    }
                });

                ui.label("Additional shapings are used within graphics_creatures_creatures_layered.txt, but the complete list is not readily prepared.");

                ui.add_space(PADDING);
                ui.label("requires a CONDITION_TISSUE_LAYER above.");
            }
            Condition::TissueNotShaped => {
                ui.add_space(PADDING);
                ui.label("requires a CONDITION_TISSUE_LAYER above.");
                ui.label("No additional input needed.");
            }
            Condition::TissueSwap(app_mod, amount, tile, [x1,y1], large_coords) => {
                let (tile_names, max_coords) = DFGraphicsHelper::tile_read(&tile_info, &tile);
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

                ui.add_space(PADDING);
                let mut large = large_coords.is_some();
                ui.checkbox(&mut large, "Large Image:");

                if large {
                    let [x2, y2] = large_coords.get_or_insert([0, 0]);
                    ui.add(egui::Slider::new(x1, 0..=max_coords[0]-*x2).prefix("Tile X: "));
                    ui.add(egui::Slider::new(y1, 0..=max_coords[1]-*y2).prefix("Tile Y: "));

                    ui.add(egui::Slider::new(x2, 0..=2).prefix("X + "));
                    ui.add(egui::Slider::new(y2, 0..=1).prefix("Y + "));
                } else {
                    ui.add(egui::Slider::new(x1, 0..=max_coords[0]).prefix("Tile X: "));
                    ui.add(egui::Slider::new(y1, 0..=max_coords[1]).prefix("Tile Y: "));

                    if large_coords.is_some() {
                        large_coords.take();
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
    }

    fn export(&self) -> Result<String, Box<dyn Error>> {
        let mut out;

        match self {
            Condition::Default => {
                out = String::new();
            },
            Condition::ItemWorn(item_type, items) => {
                out = format!("\t\t\t\t[CONDITION_ITEM_WORN:");
                match item_type {
                    ItemType::ByCategory(category, equipment) => {
                        out.push_str(&format!(
                            "BY_CATEGORY:{}:{}",
                            category.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                            equipment.name()
                        ));
                    },
                    ItemType::ByToken(token, equipment) => {
                        out.push_str(&format!(
                            "BY_TOKEN:{}:{}",
                            token.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                            equipment.name()
                        ));
                    },
                    ItemType::AnyHeld(equipment) => {
                        out.push_str(&format!(
                            "ANY_HELD:{}",
                            equipment.name().with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                        ));
                    },
                    ItemType::Wield(equipment) => {
                        out.push_str(&format!(
                            "WIELD:{}",
                            equipment.name().with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                        ));
                    },
                    ItemType::None => {}
                }
                for item in items {
                    out.push_str(&format!(
                        ":{}",
                        item.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                    ));
                }
                out.push_str("]\n");
            },
            Condition::ShutOffIfItemPresent(item_type, items) => {
                out = format!("\t\t\t\t[CONDITION_ITEM_WORN:");
                match item_type {
                    ItemType::ByCategory(category, equipment) => {
                        out.push_str(&format!("BY_CATEGORY:{}:{}",
                            category.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                            equipment.name().with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                        ));
                    },
                    ItemType::ByToken(token, equipment) => {
                        out.push_str(&format!("BY_TOKEN:{}:{}",
                            token.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                            equipment.name().with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                        ));
                    },
                    ItemType::AnyHeld(equipment) => {
                        out.push_str(&format!(
                            "ANY_HELD:{}",
                            equipment.name().with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                        ));
                    },
                    ItemType::Wield(equipment) => {
                        out.push_str(&format!(
                            "WIELD:{}",
                            equipment.name().with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                        ));
                    },
                    ItemType::None => {}
                }
                for item in items {
                    out.push_str(&format!(
                        ":{}",
                        item.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                    ));
                }
                out.push_str("]\n");
            },
            Condition::Dye(color) => {
                out = format!(
                    "\t\t\t\t[CONDITION_DYE:{}]\n",
                    color.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                );
            },
            Condition::NotDyed => {
                out = format!("\t\t\t\t[CONDITION_NOT_DYED]\n");
            },
            Condition::MaterialFlag(flags) => {
                out = "\t\t\t\t[CONDITION_MATERIAL_FLAG".to_string();
                for flag in flags {
                    out.push_str(&format!(
                        ":{}",
                        flag.name().with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                    ));
                }
                out.push_str("]\n");
            },
            Condition::MaterialType(metal) => {
                out = format!(
                    "\t\t\t\t[CONDITION_MATERIAL_TYPE:METAL:{}]\n",
                    metal.name().with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                );
            },
            Condition::ProfessionCategory(professions) => {
                out = "\t\t\t\t[CONDITION_PROFESSION_CATEGORY".to_string();
                for profession in professions {
                    out.push_str(&format!(
                        ":{}",
                        profession.name().with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                    ));
                }
                out.push_str("]\n");
            },
            Condition::RandomPartIndex(id, index, max) => {
                out = format!(
                    "\t\t\t\t[CONDITION_RANDOM_PART_INDEX:{}:{}:{}]\n",
                    id.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                    index,
                    max
                );
            },
            Condition::HaulCountMin(haul_count) => {
                out = format!("\t\t\t\t[CONDITION_HAUL_COUNT_MIN:{}]\n", haul_count);
            },
            Condition::HaulCountMax(haul_count) => {
                out = format!("\t\t\t\t[CONDITION_HAUL_COUNT_MAX:{}]\n", haul_count);
            },
            Condition::Child => {
                out = format!("\t\t\t\t[CONDITION_CHILD]\n")
            },
            Condition::NotChild => {
                out = format!("\t\t\t\t[CONDITION_NOT_CHILD]\n")
            },
            Condition::Caste(caste) => {
                out = format!(
                    "\t\t\t\t[CONDITION_CASTE:{}]\n",
                    caste.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                );
            },
            Condition::Ghost => {
                out = format!("\t\t\t\t[CONDITION_GHOST]\n");
            },
            Condition::SynClass(syn_classes) => {
                out = format!("\t\t\t\t[CONDITION_SYN_CLASS");
                for syn_class in syn_classes {
                    out.push_str(&format!(
                        ":{}",
                        syn_class.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                    ));
                }
                out.push_str("]\n");
            },
            Condition::TissueLayer(category, tissue) => {
                out = format!(
                    "\t\t\t\t[CONDITION_TISSUE_LAYER:BY_CATEGORY:{}:{}]\n",
                    category.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                    tissue.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                );
            },
            Condition::TissueMinLength(length) => {
                out = format!("\t\t\t\t\t[TISSUE_MIN_LENGTH:{}]\n", length);
            },
            Condition::TissueMaxLength(length) => {
                out = format!("\t\t\t\t\t[TISSUE_MAX_LENGTH:{}]\n", length);
            },
            Condition::TissueMayHaveColor(colors) => {
                out = format!("\t\t\t\t\t[TISSUE_MAY_HAVE_COLOR");
                for color in colors {
                    out.push_str(&format!(
                        ":{}",
                        color.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                    ));
                }
                out.push_str("]\n");
            },
            Condition::TissueMayHaveShaping(shapings) => {
                out = format!("\t\t\t\t\t[TISSUE_MAY_HAVE_SHAPING");
                for shaping in shapings {
                    out.push_str(&format!(
                        ":{}",
                        shaping.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                    ));
                }
                out.push_str("]\n");
            },
            Condition::TissueNotShaped => {
                out = format!("\t\t\t\t\t[TISSUE_NOT_SHAPED]\n");
            },
            Condition::TissueSwap(app_mod, amount, tile, [x1,y1], large_coords) => {
                out = format!(
                    "\t\t\t\t\t[TISSUE_SWAP:{}:{}:{}:{}:{}",
                    app_mod.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                    amount,
                    tile.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                    x1,
                    y1,
                );

                if let Some([x2,y2]) = large_coords {
                    out.push_str(&format!(":{}:{}]\n", x1 + x2, y1 + y2));
                } else {
                    out.push_str("]\n");
                }
            },
            Condition::Custom(string) => {
                out = string.clone().with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake);
            },
        }

        Ok(out)
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

    fn export(&self, path: &path::PathBuf) -> Result<(), Box<dyn Error>> {
        let tile_page_file = fs::File::create(
            path
            .join("graphics")
            .join(format!("tile_page_{}.txt",
            self.name.clone().to_case(Case::Snake)))
        ).expect("tile page file creation should not fail");

        let mut tile_page_writer = io::LineWriter::new(tile_page_file);
        
        tile_page_writer.write_all(format!(
            "tile_page_{}\n\n[OBJECT:TILE_PAGE]\n\n",
            self.name.to_case(Case::Snake)
        ).as_bytes()).expect("should always be able to write to a fresh text file");

        for tile in self.tiles.iter() {
            tile_page_writer.write_all(tile.export()?
                .as_bytes())
                .expect("should always be able to append a text file");
        }
        
        tile_page_writer.flush().expect("should always be able to end a text file");

        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Tile {
    name: String,           //all-caps NAME of tile
    filename: String,       //file path of image.png
    image_size: [u32; 2], //size of image in pixels
    tile_size: [u32; 2],  //size of tile in pixels
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

    fn tile_menu(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.label("Tile token");
        ui.text_edit_singleline(&mut self.name);
        ui.add_space(PADDING);

        ui.label("Image file path");
        ui.horizontal(|ui| {
            ui.label("/graphics/images/");
            ui.text_edit_singleline(&mut self.filename);
            ui.label(".png");
        });
        ui.add_space(PADDING);

        ui.label("Image size (pixels)");
        ui.horizontal(|ui| {
            ui.label(format!("Width: {}", self.image_size[0]));
            ui.label(format!("Height: {}", self.image_size[1]));
        });
        ui.add_space(PADDING);

        ui.label("Tile size (pixels)");
        ui.horizontal(|ui| {
            ui.add(egui::Slider::new(&mut self.tile_size[0], 0..=64).prefix("Width: "));
            ui.add(egui::Slider::new(&mut self.tile_size[1], 0..=96).prefix("Height: "));
        });

        ui.add_space(PADDING);
        ui.label("Preview:");
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.add(egui::Label::new(self.export().expect("should always make a valid layer")).wrap(false));
        });
    }

    fn export(&self) -> Result<String, Box<dyn Error>> {
        Ok(
            format!(
                "[TILE_PAGE:{}]\n\t[FILE:images/{}.png]\n\t[TILE_DIM:{}:{}]\n\t[PAGE_DIM_PIXELS:{}:{}]\n\n",
                self.name.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                self.filename.with_boundaries(&[Boundary::Space]).to_case(Case::Snake),
                self.tile_size.get(0).expect("tile size should be populated"),
                self.tile_size.get(1).expect("tile size should be populated"),
                self.image_size.get(0).expect("image size should be populated"),
                self.image_size.get(1).expect("image size should be populated")
            )
        )
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
}

#[derive(Debug, Default, Clone, Copy)]
struct GraphicsIndices {
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

    fn from(array: [usize; 8]) -> GraphicsIndices {
        GraphicsIndices {
            tile_page_index: array[0],
            tile_index: array[1],
            creature_file_index: array[2],
            creature_index: array[3],
            layer_set_index: array[4],
            layer_group_index: array[5],
            layer_index: array[6],
            condition_index: array[7]
        }
    }
}

struct DFGraphicsHelper {
    main_window: MainWindow,
    loaded_graphics: Graphics,
    indices: GraphicsIndices,
    path: path::PathBuf,
    texture_file_name: String,
    texture: Option<egui::TextureHandle>,
    preview_image: bool,
    cursor_coords: Option<[u32; 2]>,
    context_action: Action,
    copied: ContextData,
    undo_buffer: Vec<(Graphics, GraphicsIndices)>,
    redo_buffer: Vec<(Graphics, GraphicsIndices)>,
}
impl DFGraphicsHelper {
    fn new() -> Self {
        Self {
            main_window: MainWindow::DefaultMenu,
            loaded_graphics: Graphics::new(),
            indices: GraphicsIndices::new(),
            path: path::PathBuf::new(),
            texture_file_name: String::new(),
            texture: None,
            preview_image: true,
            cursor_coords: None,
            context_action: Action::default(),
            copied: ContextData::default(),
            undo_buffer: Vec::with_capacity(1000),
            redo_buffer: Vec::with_capacity(100),
        }
    }

    fn delete(&mut self, selected: ContextData) {
        self.save_state();
        
        let graphics = &mut self.loaded_graphics;
        let indices = &mut self.indices;

        match selected {
            ContextData::TilePage(_) => {
                graphics.tile_pages.remove(indices.tile_page_index);
                if indices.tile_page_index >=1 {
                    indices.tile_page_index -= 1;
                } else {
                    self.main_window = MainWindow::TilePageDefaultMenu;
                }
            },
            ContextData::Tile(_) => {
                graphics.tile_pages
                    .get_mut(indices.tile_page_index)
                    .unwrap()
                    .tiles
                    .remove(indices.tile_index);
                if indices.tile_index >=1 {
                    indices.tile_index -= 1;
                } else {
                    self.main_window = MainWindow::TilePageMenu;
                }
            },
            ContextData::CreatureFile(_) => {
                graphics.creature_files.remove(indices.creature_file_index);
                if indices.creature_file_index >=1 {
                    indices.creature_file_index -= 1;
                } else {
                    self.main_window = MainWindow::CreatureDefaultMenu;
                }
            },
            ContextData::Creature(_) => {
                graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .unwrap()
                    .creatures
                    .remove(indices.creature_index);
                if indices.creature_index >=1 {
                    indices.creature_index -= 1;
                } else {
                    self.main_window = MainWindow::CreatureFileMenu;
                }
            },
            ContextData::LayerSet(_) => {
                graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .unwrap()
                    .creatures
                    .get_mut(indices.creature_index)
                    .unwrap()
                    .graphics_type
                    .remove(indices.layer_set_index);
                if indices.layer_set_index >=1 {
                    indices.layer_set_index -= 1;
                } else {
                    self.main_window = MainWindow::CreatureMenu;
                }
            },
            ContextData::LayerGroup(_) => {
                if let LayerSet::Layered(_, layer_groups) = graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .unwrap()
                    .creatures
                    .get_mut(indices.creature_index)
                    .unwrap()
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .unwrap() {
                    layer_groups
                    .remove(indices.layer_group_index);
                    if indices.layer_group_index >=1 {
                        indices.layer_group_index -= 1;
                    } else {
                        self.main_window = MainWindow::LayerSetMenu;
                    }
                }
            },
            ContextData::SimpleLayer(_) => {
                if let LayerSet::Simple(simple_layers) |
                    LayerSet::Statue(simple_layers) = 
                    graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .unwrap()
                    .creatures
                    .get_mut(indices.creature_index)
                    .unwrap()
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .unwrap() {
                    simple_layers
                    .remove(indices.layer_index);
                    if indices.layer_index >=1 {
                        indices.layer_index -= 1;
                    } else {
                        self.main_window = MainWindow::CreatureMenu;
                    }
                }
            },
            ContextData::Layer(_) => {
                if let LayerSet::Layered(_, layer_groups) = graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .unwrap()
                    .creatures
                    .get_mut(indices.creature_index)
                    .unwrap()
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .unwrap() {
                    layer_groups
                    .get_mut(indices.layer_group_index)
                    .unwrap()
                    .layers
                    .remove(indices.layer_index);
                    if indices.layer_index >=1 {
                        indices.layer_index -= 1;
                    } else {
                        self.main_window = MainWindow::LayerGroupMenu;
                    }
                }
            },
            ContextData::Condition(_) => {
                if let LayerSet::Layered(_, layer_groups) = graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .unwrap()
                    .creatures
                    .get_mut(indices.creature_index)
                    .unwrap()
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .unwrap() {
                    layer_groups
                    .get_mut(indices.layer_group_index)
                    .unwrap()
                    .layers
                    .get_mut(indices.layer_index)
                    .unwrap()
                    .conditions
                    .remove(indices.condition_index);
                    if indices.condition_index >=1 {
                        indices.condition_index -= 1;
                    } else {
                        self.main_window = MainWindow::LayerMenu;
                    }
                }
            },
            ContextData::None => {},
        }
        self.context_action = Action::None;
    }

    fn undo(&mut self) {
        if let Some(pop) = self.undo_buffer.pop() {
            if self.redo_buffer.len() == self.redo_buffer.capacity() {
                self.redo_buffer.remove(0);
            }
            self.redo_buffer.push((self.loaded_graphics.clone(), self.indices));

            (self.loaded_graphics, self.indices) = pop;
        }

        self.context_action = Action::None;
    }

    fn redo(&mut self) {
        if let Some(pop) = self.redo_buffer.pop() {
            if self.undo_buffer.len() == self.undo_buffer.capacity() {
                self.undo_buffer.remove(0);
            }
            self.undo_buffer.push((self.loaded_graphics.clone(), self.indices));

            (self.loaded_graphics, self.indices) = pop;
        }

        self.context_action = Action::None;
    }

    fn save_state(&mut self) {
        self.undo_buffer.push((self.loaded_graphics.clone(), self.indices));

        if !self.redo_buffer.is_empty() {
            self.redo_buffer.clear();
        }
    }

    fn copy(&mut self, selected: ContextData) {
        self.copied = ContextData::from(selected.clone());
        self.context_action = Action::None;
    }

    fn cut(&mut self, selected: ContextData) {
        self.save_state();

        self.copied = ContextData::from(selected.clone());

        self.delete(selected);
    }

    fn paste(&mut self) {
        let data = self.copied.clone();
        self.insert(data);
    }

    fn insert(&mut self, data: ContextData) {
        self.save_state();

        let graphics = &mut self.loaded_graphics;
        let indices = &mut self.indices;

        match data {
            ContextData::TilePage(tile_page) => {
                graphics.tile_pages.insert(indices.tile_page_index, tile_page.clone());
                self.main_window = MainWindow::TilePageMenu;
            },
            ContextData::Tile(tile) => {
                graphics.tile_pages
                    .get_mut(indices.tile_page_index)
                    .unwrap()
                    .tiles
                    .insert(indices.tile_index, tile.clone());
                self.main_window = MainWindow::TileMenu;
            },
            ContextData::CreatureFile(creature_file) => {
                graphics.creature_files.insert(indices.creature_file_index, creature_file.clone());
                self.main_window = MainWindow::CreatureFileMenu;
            },
            ContextData::Creature(creature) => {
                graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .unwrap()
                    .creatures
                    .insert(indices.creature_index, creature.clone());
                self.main_window = MainWindow::CreatureMenu;
            },
            ContextData::LayerSet(layer_set) => {
                graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .unwrap()
                    .creatures
                    .get_mut(indices.creature_index)
                    .unwrap()
                    .graphics_type
                    .insert(indices.layer_set_index, layer_set.clone());
                self.main_window = MainWindow::LayerSetMenu;
            },
            ContextData::LayerGroup(layer_group) => {
                if let LayerSet::Layered(_, layer_groups) = graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .unwrap()
                    .creatures
                    .get_mut(indices.creature_index)
                    .unwrap()
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .unwrap() {
                    layer_groups
                    .insert(indices.layer_group_index, layer_group.clone());
                self.main_window = MainWindow::LayerGroupMenu;
                }
            },
            ContextData::SimpleLayer(simple_layer) => {
                if let LayerSet::Simple(simple_layers) |
                    LayerSet::Statue(simple_layers) = 
                    graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .unwrap()
                    .creatures
                    .get_mut(indices.creature_index)
                    .unwrap()
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .unwrap() {
                    simple_layers
                    .insert(indices.layer_index, simple_layer.clone());
                self.main_window = MainWindow::LayerMenu;
                }
            },
            ContextData::Layer(layer) => {
                if let LayerSet::Layered(_, layer_groups) = graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .unwrap()
                    .creatures
                    .get_mut(indices.creature_index)
                    .unwrap()
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .unwrap() {
                    layer_groups
                    .get_mut(indices.layer_group_index)
                    .unwrap()
                    .layers
                    .insert(indices.layer_index, layer.clone());
                    self.main_window = MainWindow::LayerMenu;
                }
            },
            ContextData::Condition(condition) => {
                if let LayerSet::Layered(_, layer_groups) = graphics.creature_files
                    .get_mut(indices.creature_file_index)
                    .unwrap()
                    .creatures
                    .get_mut(indices.creature_index)
                    .unwrap()
                    .graphics_type
                    .get_mut(indices.layer_set_index)
                    .unwrap() {
                    layer_groups
                    .get_mut(indices.layer_group_index)
                    .unwrap()
                    .layers
                    .get_mut(indices.layer_index)
                    .unwrap()
                    .conditions
                    .insert(indices.condition_index, condition.clone());
                    self.main_window = MainWindow::ConditionMenu;
                }
            },
            ContextData::None => {},
        }
        self.context_action = Action::None;
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
        } else if ui.button("Undo").clicked() {
            ui.close_menu();
            action = Action::Undo;
        } else if ui.button("Redo").clicked() {
            ui.close_menu();
            action = Action::Redo;
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
                if ui.add(egui::Label::new(
                    format!("Tile Page: {}", &tile_page.name))
                    .sense(Sense::click()))
                    .context_menu(|ui| {
                    self.indices = GraphicsIndices::from([i_tile_page, 0, 0, 0, 0, 0, 0, 0]);
                    self.context_action = Self::context(ui, ContextData::from(tile_page.clone()));
                }).clicked() {
                    self.indices = GraphicsIndices::from([i_tile_page, 0, 0, 0, 0, 0, 0, 0]);
                    self.main_window = MainWindow::TilePageMenu;
                }
            })
            .body(|ui| {
                for (i_tile, tile) in tile_page.tiles.iter_mut().enumerate() {
                    if ui.add(egui::Label::new(&tile.name).sense(Sense::click()))
                        .context_menu(|ui| {
                        self.indices = GraphicsIndices::from([i_tile_page, i_tile, 0, 0, 0, 0, 0, 0]);
                        self.context_action = Self::context(ui, ContextData::from(tile.clone()));
                    }).clicked() {
                        self.indices = GraphicsIndices::from([i_tile_page, i_tile, 0, 0, 0, 0, 0, 0]);
                        self.main_window = MainWindow::TileMenu;
                    }
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
                if ui.add(egui::Label::new(
                    format!("File: {}", &creature_file.name))
                    .sense(Sense::click()))
                    .context_menu(|ui| {
                    self.indices = GraphicsIndices::from([0, 0, i_file, 0, 0, 0, 0, 0]);
                    self.context_action = Self::context(ui, ContextData::from(creature_file.clone()));
                }).clicked() {
                    self.indices = GraphicsIndices::from([0, 0, i_file, 0, 0, 0, 0, 0]);
                    self.main_window = MainWindow::CreatureFileMenu;
                };
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
                        if ui .add(egui::Label::new(
                            format!("Creature: {}", &creature.name))
                            .sense(Sense::click()))
                            .context_menu(|ui| {
                            self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, 0, 0, 0, 0]);
                            self.context_action = Self::context(ui, ContextData::from(creature.clone()));
                        }).clicked() {
                            self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, 0, 0, 0, 0]);
                            self.main_window = MainWindow::CreatureMenu;
                        };
                    })
                    .body(|ui| {
                        for (i_layer_set, layer_set) in creature.graphics_type.iter_mut().enumerate() {
                            match layer_set {
                                LayerSet::Empty => {
                                    if ui.add(egui::Label::new("(empty)")
                                        .sense(Sense::click()))
                                        .context_menu(|ui| {
                                        self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, 0, 0, 0]);
                                        self.context_action = Self::context(ui, ContextData::from(layer_set.clone()));
                                    }).clicked() {
                                        self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, 0, 0, 0]);
                                        self.main_window = MainWindow::LayerSetMenu;
                                    }
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
                                        if ui.add(egui::Label::new(
                                            format!("Set: {}", state.name()))
                                            .sense(Sense::click()))
                                            .context_menu(|ui| {
                                            self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, 0, 0, 0]);
                                            self.context_action = Self::context(ui, ContextData::from(LayerSet::Layered(state.clone(), layer_groups.clone())));
                                        }).clicked() {
                                            self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, 0, 0, 0]);
                                            self.main_window = MainWindow::LayerSetMenu;
                                        }
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
                                                true)
                                                .show_header(ui, |ui|
                                                {
                                                if ui.add(egui::Label::new(
                                                    format!("Group: {}", &layer_group.name))
                                                    .sense(Sense::click())).context_menu(|ui| {
                                                    self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, i_layer_group, 0, 0]);
                                                    self.context_action = Self::context(ui, ContextData::from(layer_group.clone()));
                                                }).clicked() {
                                                    self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, i_layer_group, 0, 0]);
                                                    self.main_window = MainWindow::LayerGroupMenu;
                                                };
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
                                                        if ui.add(egui::Label::new(
                                                            format!("Layer: {}", &layer.name))
                                                            .sense(Sense::click()))
                                                            .context_menu(|ui| {
                                                            self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, i_layer_group, i_layer, 0]);
                                                            self.context_action = Self::context(ui, ContextData::from(layer.clone()));
                                                        }).clicked() {
                                                            self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, i_layer_group, i_layer, 0]);
                                                            self.main_window = MainWindow::LayerMenu;
                                                        }
                                                    })
                                                        .body(|ui|
                                                        {
                                                        for (i_condition, condition) in layer.conditions.iter_mut().enumerate() {
                                                            if ui.add(egui::Label::new(condition.name())
                                                                .sense(Sense::click()))
                                                                .context_menu(|ui| {
                                                                self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, i_layer_group, i_layer, i_condition]);
                                                                self.context_action = Self::context(ui, ContextData::from(condition.clone()));
                                                            }).clicked() {
                                                                self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, i_layer_group, i_layer, i_condition]);
                                                                self.main_window = MainWindow::ConditionMenu;
                                                            }
                                                        }
                                                    });
                                                }
                                            });
                                        }
                                    });
                                },
                                LayerSet::Simple(simple_layers) => {
                                    for (i_layer, simple_layer) in simple_layers.iter_mut().enumerate() {
                                        if ui.add(egui::Label::new(
                                        if let Some(sub_state) = &simple_layer.sub_state {
                                                format!("\t{} & {}",
                                                simple_layer.state.name(),
                                                sub_state.name())
                                            } else {
                                                format!("\t{}",
                                                simple_layer.state.name())
                                            })
                                            .sense(Sense::click()))
                                            .context_menu(|ui| {
                                            self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, 0, i_layer, 0]);
                                            self.context_action = Self::context(ui, ContextData::from(simple_layer.clone()));
                                        }).clicked() {
                                            self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, 0, i_layer, 0]);
                                            self.main_window = MainWindow::LayerMenu;
                                        }
                                    }
                                },
                                LayerSet::Statue(simple_layers) => {
                                    for (i_layer, simple_layer) in simple_layers.iter_mut().enumerate() {
                                        if ui.add(egui::Label::new(
                                        if let Some(sub_state) = &simple_layer.sub_state {
                                                format!("\tStatue: {} & {}",
                                                simple_layer.state.name(),
                                                sub_state.name())
                                            } else {
                                                format!("\tStatue: {}",
                                                simple_layer.state.name())
                                            })
                                            .sense(Sense::click()))
                                            .context_menu(|ui| {
                                            self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, 0, i_layer, 0]);
                                            self.context_action = Self::context(ui, ContextData::from(simple_layer.clone()));
                                        }).clicked() {
                                            self.indices = GraphicsIndices::from([0, 0, i_file, i_creature, i_layer_set, 0, i_layer, 0]);
                                            self.main_window = MainWindow::LayerMenu;
                                        }
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

    fn default_menu(&mut self, ui: &mut Ui) {
        ui.label("Welcome!");
        ui.separator();

        ui.add_space(PADDING);
        ui.hyperlink_to(
            "DF Graphics Helper on GitHub",
            "https://github.com/BarelyCreative/DF-graphics-helper/tree/main",
        );
    }

    fn tile_page_default_menu(&mut self, ui: &mut Ui) {
        ui.label("Tile Page Menu");
        ui.separator();

        if ui.small_button("New Tile Page").clicked() {
            self.context_action = Action::Insert(ContextData::TilePage(TilePage::new()));
        }
    }

    fn creature_default_menu(&mut self, ui: &mut Ui) {
        ui.label("Creature File Menu");
        ui.separator();

        if ui.small_button("New Creature File").clicked() {
            self.context_action = Action::Insert(ContextData::CreatureFile(CreatureFile::new()));
        }
    }

    fn creature_file_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Creature File Menu");
            if ui.button("Delete").clicked() {
                self.context_action = Action::Delete(ContextData::CreatureFile(CreatureFile::new()));
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
                .unwrap();

            ui.separator();
            ui.text_edit_singleline(&mut creature_file.name);
            ui.add_space(PADDING);

            if ui.button("New Creature").clicked() {
                self.context_action = Action::Insert(ContextData::Creature(Creature::new()));
            }
        }
    }

    fn creature_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Creature Menu");
            if ui.button("Delete").clicked() {
                self.context_action = Action::Delete(ContextData::Creature(Creature::new()));
            }
        });

        let indices = &mut self.indices;

        let creatures = &mut self
            .loaded_graphics
            .creature_files
            .get_mut(indices.creature_file_index)
            .unwrap()
            .creatures;
        
        if creatures.is_empty() {
            if ui.small_button("Create Creature").clicked() {
                self.context_action = Action::Insert(ContextData::Creature(Creature::new()));
            }
        } else {
            let creature = creatures
                .get_mut(indices.creature_index)
                .unwrap();

            creature.creature_menu(ui);
        }
    }

    fn layer_set_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Layer Set Menu");
            if ui.button("Delete").clicked() {
                self.context_action = Action::Delete(ContextData::LayerSet(LayerSet::Simple(Vec::new())));
            }
        });

        let indices = &mut self.indices;

        let layer_sets = &mut self
            .loaded_graphics
            .creature_files
            .get_mut(indices.creature_file_index)
            .unwrap()
            .creatures
            .get_mut(indices.creature_index)
            .unwrap()
            .graphics_type;
        
        if layer_sets.is_empty() {
            if ui.small_button("Create Layer Set").clicked() {
                self.context_action = Action::Insert(ContextData::LayerSet(LayerSet::default()));
            }
        } else {
            let layer_set = layer_sets
                .get_mut(indices.layer_set_index)
                .unwrap();

            layer_set.layer_set_menu(ui);
        }
    }

    fn tile_page_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Tile Page Menu");
            if ui.button("Delete").clicked() {
                self.context_action = Action::Delete(ContextData::TilePage(TilePage::new()));
            }
        });

        let indices = &mut self.indices;

        if self.loaded_graphics.tile_pages.is_empty() {
            self.context_action = Action::Insert(ContextData::TilePage(TilePage::new()));
        } else {
            let tile_page = self
                .loaded_graphics
                .tile_pages
                .get_mut(indices.tile_page_index)
                .unwrap();

            ui.separator();
            ui.text_edit_singleline(&mut tile_page.name);
            ui.add_space(PADDING);

            if ui.button("New Tile").clicked() {
                self.context_action = Action::Insert(ContextData::Tile(Tile::new()));
            }
        }
    }

    fn tile_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Tile Menu");
            if ui.button("Delete").clicked() {
                self.context_action = Action::Delete(ContextData::Tile(Tile::new()));
            }
        });
        
        let indices = &mut self.indices;

        let tiles = &mut self
            .loaded_graphics
            .tile_pages
            .get_mut(indices.tile_page_index)
            .unwrap()
            .tiles;

        if tiles.is_empty() {
            if ui.small_button("Create Tile").clicked() {
                self.context_action = Action::Insert(ContextData::Tile(Tile::new()));
            }
        } else {
            let tile = tiles.get_mut(indices.tile_index).unwrap();
            let file_name = tile.filename.clone();

            tile.tile_menu(ui);

            ui.add_space(PADDING);
            self.preview_image(ui, file_name, None);
        }
    }

    fn layer_group_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Layer Group Menu");
            if ui.button("Delete").clicked() {
                self.context_action = Action::Delete(ContextData::LayerGroup(LayerGroup::new()));
            }
        });
        
        let indices = &mut self.indices;

        let graphics_type = self
            .loaded_graphics
            .creature_files
            .get_mut(indices.creature_file_index)
            .unwrap()
            .creatures
            .get_mut(indices.creature_index)
            .unwrap()
            .graphics_type
            .get_mut(indices.layer_set_index)
            .unwrap();

        if let LayerSet::Layered(_, layer_groups) = graphics_type {
            if layer_groups.is_empty() {
                if ui.small_button("Create Layer Group").clicked() {
                    self.context_action = Action::Insert(ContextData::LayerGroup(LayerGroup::new()));
                }
            } else {
                let layer_group = layer_groups
                    .get_mut(indices.layer_group_index)
                    .unwrap();

                layer_group.layer_group_menu(ui);
            }
        }
    }

    fn layer_menu(&mut self, ui: &mut Ui) {
        let tile_info = self.tile_info();

        let cursor_coords = self.cursor_coords;
        
        let indices = &mut self.indices;

        let layer_groups = self
            .loaded_graphics
            .creature_files
            .get_mut(indices.creature_file_index)
            .unwrap()
            .creatures
            .get_mut(indices.creature_index)
            .unwrap()
            .graphics_type
            .get_mut(indices.layer_set_index)
            .unwrap();

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
                        self.context_action = Action::Insert(ContextData::SimpleLayer(SimpleLayer::new()));
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Layer Menu");
                        if ui.button("Delete").clicked() {
                            self.context_action = Action::Delete(ContextData::SimpleLayer(SimpleLayer::new()));
                        }
                    });

                    let simple_layer = simple_layers.get_mut(indices.layer_index).unwrap();

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
                    self.preview_image(ui, file_name, rect);
                }
            },
            LayerSet::Statue(simple_layers) => {
                if simple_layers.is_empty() {
                    //if there are no layers defined show create layer button only
                    ui.label("Statue Layer Menu");
                    ui.separator();
                    if ui.small_button("Create Layer").clicked() {
                        self.context_action = Action::Insert(ContextData::SimpleLayer(SimpleLayer::new()));
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Layer Menu");
                        if ui.button("Delete").clicked() {
                            self.context_action = Action::Delete(ContextData::SimpleLayer(SimpleLayer::new()));
                        }
                    });
                    
                    let simple_layer = simple_layers.get_mut(indices.layer_index).unwrap();

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
                    self.preview_image(ui, file_name, rect);
                }
            },
            LayerSet::Layered(_, layer_groups) => {
                let layers = 
                    &mut layer_groups
                    .get_mut(indices.layer_group_index)
                    .unwrap()
                    .layers;

                if layers.is_empty() {
                    //if there are no layers defined show create layer button only
                    ui.label("Layer Menu");
                    ui.separator();
                    if ui.small_button("Create Layer").clicked() {
                        self.context_action = Action::Insert(ContextData::Layer(Layer::new()));
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Layer Menu");
                        if ui.button("Delete").clicked() {
                            self.context_action = Action::Delete(ContextData::Layer(Layer::new()));
                        }
                    });

                    let layer = layers.get_mut(indices.layer_index).unwrap();

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
                    self.preview_image(ui, file_name, rect);
                }

            },
            LayerSet::Empty => {
                ui.horizontal(|ui| {
                    ui.label("Empty Layer Menu");
                    if ui.button("Delete").clicked() {
                        self.context_action = Action::Delete(ContextData::Layer(Layer::new()));
                    }
                });
            },
        }
    }

    fn preview_image(&mut self, ui: &mut Ui, file_name: String, rectangle: Option<[[u32; 2]; 2]>) {
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
                    "{}, {}",
                    (val.x / 32.0).floor(),
                    (val.y / -32.0).floor()
                )
            };

            let plot = Plot::new("image_preview")
                .auto_bounds_x()
                .auto_bounds_y()
                .data_aspect(1.0)
                .show_background(true)
                .allow_boxed_zoom(false)
                .clamp_grid(true)
                .min_size(egui::vec2(100.0, 400.0))
                .set_margin_fraction(egui::vec2(0.005, 0.005))
                .x_axis_formatter(x_fmt)
                .y_axis_formatter(y_fmt)
                .x_grid_spacer(Self::grid)
                .y_grid_spacer(Self::grid)
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

                    let rectangle = egui::plot::Polygon::new(points)
                        .color(egui::Color32::LIGHT_BLUE)
                        .fill_alpha(0.01);
                    plot_ui.polygon(rectangle);
                }
                self.cursor_coords.take();
                if plot_ui.plot_secondary_clicked() {
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
                let dyn_image = image::open(image_path).unwrap();
                let size = [dyn_image.width() as _, dyn_image.height() as _];
                let image = dyn_image.as_bytes();
                let rgba = egui::ColorImage::from_rgba_unmultiplied(size, image);

                self.loaded_graphics.tile_pages
                    .get_mut(self.indices.tile_page_index)
                    .unwrap()
                    .tiles
                    .get_mut(self.indices.tile_index)
                    .unwrap()
                    .image_size = [dyn_image.width(), dyn_image.height()];
                
                self.texture.get_or_insert_with(|| {
                    ui.ctx()
                        .load_texture("default_image", rgba, Default::default())
                });
                self.texture_file_name = file_name;
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

            marks.push(egui::plot::GridMark {
                value: i as f64,
                step_size,
            });
        }

        marks
    }

    fn condition_menu(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Condition Menu");
            if ui.button("Delete").clicked() {
                self.context_action = Action::Delete(ContextData::Condition(Condition::default()));
            }
        });

        let tile_info = self.tile_info();
        
        let indices = &mut self.indices;

        let graphics_type = self
            .loaded_graphics
            .creature_files
            .get_mut(indices.creature_file_index)
            .unwrap()
            .creatures
            .get_mut(indices.creature_index)
            .unwrap()
            .graphics_type
            .get_mut(indices.layer_set_index)
            .unwrap();

        if let LayerSet::Layered(_, layergroups) = graphics_type {
            let conditions = &mut layergroups
                .get_mut(indices.layer_group_index)
                .unwrap()
                .layers
                .get_mut(indices.layer_index)
                .unwrap()
                .conditions;

            if conditions.is_empty() {
                if ui.small_button("New condition").clicked() {
                    self.context_action = Action::Insert(ContextData::Condition(Condition::default()));
                }
            } else {
                let condition = conditions
                    .get_mut(indices.condition_index)
                    .unwrap();

                ui.separator();
    
                condition.condition_menu(ui, tile_info);
            }
        }
    }

    fn tile_info(&self) -> Vec<(String, [u32; 2])> {
        let mut tile_info: Vec<(String, [u32; 2])> = self
            .loaded_graphics
            .tile_pages
            .iter()
            .flat_map(|tile_page| {
                tile_page.tiles.iter().map(|t| {
                    if t.tile_size[0]*t.tile_size[1]*t.image_size[0]*t.image_size[1] != 0 {
                        (t.name.clone(),
                        [t.image_size[0]/t.tile_size[0] - 1,
                        t.image_size[1]/t.tile_size[1] - 1])
                    } else{
                        (t.name.clone(), [100, 100])
                    }
                })
            })
            .collect();

        tile_info.sort();
        tile_info.dedup();
        
        tile_info
    }

    fn tile_read(tile_info: &Vec<(String, [u32; 2])>, name: &String) -> (Vec<String>, [u32; 2]) {
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
                        if let Some(mut path) = rfd::FileDialog::new()
                            .set_title(r"Choose Mod Folder")
                            .pick_folder() {
                            (self.loaded_graphics, self.path) = Graphics::import(&mut path).unwrap();
                        }
                        self.save_state();
                        ui.close_menu();
                    }
                    if ui.button("Import").clicked() {
                        if !self.path.exists() {
                            if let Some(mut path) = rfd::FileDialog::new()
                                .set_title(r"Choose Mod Folder")
                                .pick_folder() {
                                (self.loaded_graphics, self.path) = Graphics::import(&mut path).unwrap();
                            }
                        } else {
                            (self.loaded_graphics, self.path) = Graphics::import(&mut self.path).unwrap();
                        }
                        self.save_state();
                        ui.close_menu();
                    }
                    if ui.button("Export").clicked() {
                        if !self.path.exists() {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_title(r"Choose Mod Folder")
                                .pick_folder() {
                                self.loaded_graphics.export(&path).unwrap();
                            }
                        } else {
                            self.loaded_graphics.export(&self.path.clone()).unwrap();
                        }
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
                    self.main_tree(ui, ctx);
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            //Draw main window by matching self.main_window
            egui::ScrollArea::horizontal()
                .show(ui, |ui| 
                match self.main_window {
                    MainWindow::TilePageDefaultMenu => self.tile_page_default_menu(ui),
                    MainWindow::CreatureDefaultMenu => self.creature_default_menu(ui),
                    MainWindow::TilePageMenu => self.tile_page_menu(ui),
                    MainWindow::TileMenu => self.tile_menu(ui),
                    MainWindow::CreatureFileMenu => self.creature_file_menu(ui),
                    MainWindow::CreatureMenu => self.creature_menu(ui),
                    MainWindow::LayerSetMenu => self.layer_set_menu(ui),
                    MainWindow::LayerGroupMenu => self.layer_group_menu(ui),
                    MainWindow::LayerMenu => self.layer_menu(ui),
                    MainWindow::ConditionMenu => self.condition_menu(ui),
                    MainWindow::ReferenceMenu => self.default_menu(ui),
                    MainWindow::DefaultMenu => self.default_menu(ui),
                }
            );
        });

        match &self.context_action { //respond to the context menus
            Action::Delete(selected) => {
                self.delete(selected.clone())
            },
            Action::Copy(selected) => {
                self.copy(selected.clone());
            },
            Action::Cut(selected) => {
                self.cut(selected.clone());
            },
            Action::Paste => {
                self.paste();
            },
            Action::Duplicate(selected) => {
                self.copy(selected.clone());
                self.paste();
            },
            Action::Insert(kind) => {
                self.insert(kind.clone());
            },
            Action::Undo => {
                self.undo();
            },
            Action::Redo => {
                self.redo();
            },
            Action::None => {},
        }
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