use std::ffi::OsStr;
use std::path;
use std::io::prelude::*;
use std::{fs, io};
use convert_case::{Boundary, Case, Casing};
use egui::Ui;

pub mod logic;
use logic::app::DFGraphicsHelper;
use logic::error::{Result, DFGHError, wrap_import_error, wrap_block_error};

pub const PADDING: f32 = 8.0;
// i_b_line: usize, r_elem: RangeInclusive<usize>, raw_buffer: Vec<String>, e: DFGHError
macro_rules! block_err_wrap {
    ($func:expr, $i_b_line:ident, $raw_buffer:ident, $r_elem:expr) => {
        match $func {
            Ok(inner) => inner,
            //i_b_line: usize, r_elem: RangeInclusive<usize>, raw_buffer: Vec<String>, e: DFGHError
            Err(e) => return wrap_block_error($i_b_line, $r_elem, $raw_buffer, DFGHError::from(e)),
        }
    };
}

macro_rules! index_err {
    ($i_b_line:ident, $len:ident, $expected:expr, $raw_buffer:ident, $type:ty) => {
        let e = DFGHError::ImportIndexError($expected, $len);

        if $len < $expected {
            return Err::<$type, DFGHError>(DFGHError::ImportBlockError($i_b_line, $i_b_line, 0..=$len, $raw_buffer, e.to_string()))
        } else {
            return Err::<$type, DFGHError>(DFGHError::ImportBlockError($i_b_line, $i_b_line, $expected..=$len, $raw_buffer, e.to_string()))
        }
    };
}

macro_rules! ls_push {
    ($creature:ident, $graphics_type:ident, $ls_buffer:ident, $i_b_line:ident, $raw_buffer:ident) => {
        if let LayerSet::Layered(state, _) = $graphics_type {
            if !$ls_buffer.is_empty() {
                $creature.graphics_type.push(
                    block_err_wrap!(
                        Self::import_layer_set(state, $ls_buffer.clone(), $raw_buffer.clone()),
                        $i_b_line, $raw_buffer, 0..=0
                    )
                );
                $ls_buffer.clear();
            }
        }
    }
}

macro_rules! lg_push {
    ($layer_groups:ident, $lg_buffer:ident, $i_b_line:ident, $raw_buffer:ident) => {
        if !$lg_buffer.is_empty() {
            $layer_groups.push(
                block_err_wrap!(
                    Self::import_layer_group($lg_buffer.clone(), $raw_buffer.clone()),
                    $i_b_line, $raw_buffer, 0..=0
                )
            );
            $lg_buffer.clear();
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Graphics {
    pub tile_pages: Vec<TilePage>,
    pub creature_files: Vec<CreatureFile>,
}
impl Graphics {
    /// Generate a blank generic Graphics struct
    pub fn new() -> Graphics {
        Graphics {
            tile_pages: vec![TilePage::new()],
            creature_files: vec![CreatureFile::new()],
        }
    }

    /// Take a line loaded from a text file and filter out the valid arguements between '[]'s into a vector.
    /// 
    /// Returns a true if there were valid brackets, a vector of the internal arguments, and any text after the closing bracket as an optional comment.
    /// ```
    /// # use df_texture_helper::*;
    /// let commented_line = " ignored pre-text \t \t[CREATURE:DWARF] ignored comment text ".to_string();
    /// let line_vec = Graphics::read_brackets(&commented_line);
    /// 
    /// assert_eq!("CREATURE", line_vec[0]);
    /// assert_eq!("DWARF", line_vec[1]);
    /// assert_eq!(2, line_vec.len());
    /// 
    /// let empty_line = " Lorem ipsum dolor".to_string();
    /// assert!(Graphics::read_brackets(&empty_line).is_empty());
    /// ```
    pub fn read_brackets(raw_line: &String) -> Vec<String> {
        // let mut split_line: Vec<&str> = 
        
        let line_vec: Vec<String>;
        if let Some((_, open_brac_line)) = raw_line.split_once('[') {
            if let Some((valid_line, _)) = open_brac_line.split_once(']') {
                line_vec = valid_line.split(':').map(|s| s.to_string()).collect();
            } else {
                line_vec = Vec::new();
            }
        } else {
            line_vec = Vec::new();
        }

        line_vec
    }

    /// Loads a graphics directory into the program.
    /// 
    /// Identifies if the selected path contains or is inside a valid graphics directory, 
    /// then attempts to each graphics or tilepage file in the graphics directory.
    /// 
    /// ```
    /// # use df_texture_helper::*;
    /// let mut folder = std::path::PathBuf::from("C:\\Users\\Riley\\Desktop\\DwarfFortress\\Mod Dev Files\\DF Graphics Helper\\df_graphics_helper");
    ///
    /// Graphics::import(&mut folder);
    /// ```
    pub fn import(folder: &mut path::PathBuf) -> Result<(Graphics, path::PathBuf)> {
        let mut tile_pages: Vec<TilePage> = Vec::new();
        let mut creature_files: Vec<CreatureFile> = Vec::new();

        //Check if the path includes or is inside a graphics directory and adjust path to show full mod folder.
        if folder.ends_with("graphics") {
            folder.pop();
        } else if folder.ends_with("images") && folder.parent().get_or_insert(path::Path::new("")).ends_with("graphics") {
            folder.pop();
            folder.pop();
        } else if !folder.read_dir()?.any(|f| f.is_ok_and(|f| f.path().ends_with("graphics"))) {
            //if no graphics directory in mod folder throw error.
            return Err(DFGHError::NoGraphicsDirectory(folder.clone()));
        }

        //read graphics directory from mod folder.
        let paths = fs::read_dir(&folder.join("graphics"))?;

        //read each tile page or creature graphics text file and import.
        for path in paths {
            let path = path?;
            if let Ok(entry_name) = path.file_name().into_string() {
                if entry_name.ends_with(".txt") {
                    if entry_name.starts_with("tile_page_") {
                        //import tile page file
                        tile_pages.push(Self::import_tile_page(&path.path())?);
    
                    } else if entry_name.starts_with("graphics_creatures_") {
                        //import creature file
                        creature_files.push(Self::import_creature_file(&path.path())?);
                    }
                }
            } else {
                return Err(DFGHError::UnsupportedFileName(path.path()))
            }
        }

        Ok(
            (
                Graphics {
                    tile_pages,
                    creature_files,
                    ..Default::default()
                },
                folder.clone()
            )
        )
    }

    fn import_tile_page(path: &path::PathBuf) -> Result<TilePage> {
        let mut tiles = Vec::new();
        let mut buffer = Vec::with_capacity(10);
        let mut raw_buffer = Vec::with_capacity(10);
        let mut buffer_start = 0;

        let f = fs::File::open(path)?;

        let lines = io::BufReader::new(f).lines()
            .map(|l| l.expect("should always be a valid line."));

        //first line must match file name and is tile page name.
        let name = path
            .file_name().get_or_insert(&OsStr::new("no_name"))
            .to_str().get_or_insert("no_name")
            .replace("tile_page_", "")
            .replace(".txt", "").trim().to_string();
        
        //read line-by-line to find starts of all tile definitions.
        //create vectors of all lines between tile headers and import each vector.
        for (i_line, raw_line) in lines.enumerate() {
            let line_vec = Self::read_brackets(&raw_line);

            //start filling the buffer at tile start, then process and clear it at next tile start.
            if line_vec.get(0).is_some() {
                if line_vec[0].eq("TILE_PAGE") {
                    if buffer.len() > 0 {
                        //if the buffer is populated process/clear it and store.
                        // wrap_import_error<T>(e: DFGHError, buffer_start: usize, path: &path::PathBuf) -> Result<T> {
                        match Self::import_tile(buffer.clone(), raw_buffer.clone(), path) {
                            Ok(tile) => tiles.push(tile),
                            Err(e) => return wrap_import_error(e, buffer_start, path),
                        }
                        raw_buffer.clear();
                    }
                    buffer_start = i_line;
                    buffer.push(line_vec);
                    raw_buffer.push(raw_line);
                } else if line_vec[0].eq("OBJECT") {
                    //do nothing
                } else {
                    buffer.push(line_vec);
                    raw_buffer.push(raw_line);
                }
            }
        }

        Ok(TilePage {name, tiles})
    }

    ///Takes a chunk of processed lines and generates a tile from them.
    fn import_tile(buffer: Vec<Vec<String>>, raw_buffer: Vec<String>, path: &path::PathBuf) -> Result<Tile> {
        let mut tile = Tile::new();

        for (i_b_line, line_vec) in buffer.iter().enumerate() {
            let len = line_vec.len();
            match line_vec[0].as_str() {
                "TILE_PAGE" => {
                    if len >= 2 {
                        tile.name = line_vec[1].clone();
                    } else {
                        index_err!(i_b_line, len, 2, raw_buffer, Tile);
                    }
                },
                "FILE" => {
                    if len >= 2 {
                        tile.filename = line_vec[1].clone()
                            .replace(".png", "")
                            .replace("images", "")
                            .split_off(1);
                    } else {
                        index_err!(i_b_line, len, 2, raw_buffer, Tile);
                    }
                },
                //($func:expr, $i_b_line:ident, $raw_buffer:ident, $r_elem:expr)
                "TILE_DIM" => {
                    if len >= 3 {
                        tile.tile_size = 
                            [block_err_wrap!(line_vec[1].parse(), i_b_line, raw_buffer, 1..=1),
                            block_err_wrap!(line_vec[2].parse(), i_b_line, raw_buffer, 2..=2)];
                    } else {
                        index_err!(i_b_line, len, 3, raw_buffer, Tile);
                    }
                },
                "PAGE_DIM_PIXELS" => {
                    // if the image file name is already read attempt to correct the image size based on it.
                    if !tile.filename.is_empty() {
                        let mod_path = path.to_path_buf();
                        let image_path = mod_path.join("graphics").join("images").join(&tile.filename);
                        if image_path.is_file() && image_path.extension() == Some(OsStr::new("png")) {
                            let image_dimensions = block_err_wrap!(image::image_dimensions(image_path), i_b_line, raw_buffer, 0..=len);
                            tile.image_size = [image_dimensions.0, image_dimensions.1];
                        }
                        continue;
                    }

                    //if image file is not read yet, attempt to parse tag.
                    if len >= 3 {
                        tile.image_size = 
                            [block_err_wrap!(line_vec[1].parse(), i_b_line, raw_buffer, 1..=1),
                            block_err_wrap!(line_vec[2].parse(), i_b_line, raw_buffer, 2..=2)];
                    } else {
                        index_err!(i_b_line, len, 3, raw_buffer, Tile);
                    }
                },
                _ => {
                    return Err::<Tile, DFGHError>(DFGHError::ImportBlockError(i_b_line, i_b_line, 0..=len, raw_buffer, DFGHError::ImportUnknownError.to_string()))
                },
            }
        }

        Ok(tile)
    }

    fn import_creature_file(path: &path::PathBuf) -> Result<CreatureFile> {
        let mut creatures = Vec::new();
        let mut buffer = Vec::with_capacity(10);
        let mut raw_buffer = Vec::with_capacity(10);
        let mut buffer_start = 0;

        let f = fs::File::open(path)?;

        let lines = io::BufReader::new(f).lines()
            .map(|l| l.expect("should always be a valid line."));

        //first line must match file name and is tile page name.
        let name = path
            .file_name().get_or_insert(&OsStr::new("no_name"))
            .to_str().get_or_insert("no_name")
            .replace("graphics_creatures_", "")
            .replace(".txt", "").trim().to_string();
    
        //read line-by-line to find starts of all tile definitions.
        //create vectors of all lines between tile headers and import each vector.
        for (i_line, raw_line) in lines.enumerate() {
            let line_vec = Self::read_brackets(&raw_line);

            //start filling the buffer at tile start, then process and clear it at next tile start.
            match line_vec.get(0).unwrap_or(&"".to_string()).as_str() {
                "CREATURE_GRAPHICS"
                | "CREATURE_CASTE_GRAPHICS"
                | "STATUE_CREATURE_GRAPHICS"
                | "STATUE_CREATURE_CASTE_GRAPHICS" => {
                    if buffer.len() > 0 {
                        //if the buffer is populated process/clear it and store.
                        match Self::import_creature(buffer.clone(), raw_buffer.clone()) {
                            Ok(creature) => creatures.push(creature),
                            Err(e) => return wrap_import_error(e, buffer_start, path),
                        }
                        buffer.clear();
                        raw_buffer.clear();
                    }
                    buffer_start = i_line;
                    buffer.push(line_vec);
                    raw_buffer.push(raw_line);
                },
                "OBJECT"
                | "" => {
                    //do nothing
                },
                _ => {
                    buffer.push(line_vec);
                    raw_buffer.push(raw_line);
                }
            }
        }

        if buffer.len() > 0 {
            //if the buffer is populated process/clear it and store.
            match Self::import_creature(buffer.clone(), raw_buffer.clone()) {
                Ok(creature) => creatures.push(creature),
                Err(e) => return wrap_import_error(e, buffer_start, path),
            }
        }

        Ok(CreatureFile{
            name,
            creatures,
        })
    }

    fn import_creature(buffer: Vec<Vec<String>>, raw_buffer: Vec<String>) -> Result<Creature> {
        let mut creature = Creature::new();
        let mut graphics_type = LayerSet::Empty;
        let mut caste = None;
        let mut ls_buffer = Vec::new();

        for (i_b_line, line_vec) in buffer.iter().enumerate() {
            let len = line_vec.len();
            match line_vec[0].as_str() {
                "CREATURE_GRAPHICS" => {
                    ls_push!(creature, graphics_type, ls_buffer, i_b_line, raw_buffer);
                    if len >= 2 {
                        creature.name = line_vec[1].clone();
                        graphics_type = LayerSet::Simple(Vec::new());
                    } else {
                        index_err!(i_b_line, len, 2, raw_buffer, Creature);
                    }
                },
                "CREATURE_CASTE_GRAPHICS" => {
                    ls_push!(creature, graphics_type, ls_buffer, i_b_line, raw_buffer);
                    if len >= 3 {
                        creature.name = line_vec[1].clone();
                        caste = Some(State::from(line_vec[2].clone()));
                        graphics_type = LayerSet::Simple(Vec::new());
                    } else {
                        index_err!(i_b_line, len, 3, raw_buffer, Creature);
                    }
                },
                "STATUE_CREATURE_GRAPHICS" => {
                    ls_push!(creature, graphics_type, ls_buffer, i_b_line, raw_buffer);
                    if len >= 2 {
                        creature.name = line_vec[1].clone();
                        graphics_type = LayerSet::Statue(Vec::new());
                    } else {
                        index_err!(i_b_line, len, 2, raw_buffer, Creature);
                    }
                },
                "STATUE_CREATURE_CASTE_GRAPHICS" => {
                    ls_push!(creature, graphics_type, ls_buffer, i_b_line, raw_buffer);
                    if len >= 3 {
                        creature.name = line_vec[1].clone();
                        caste = Some(State::from(line_vec[2].clone()));
                        graphics_type = LayerSet::Statue(Vec::new());
                    } else {
                        index_err!(i_b_line, len, 3, raw_buffer, Creature);
                    }
                },
                "LAYER_SET" => {
                    ls_push!(creature, graphics_type, ls_buffer, i_b_line, raw_buffer);
                    if len >= 2 {
                        graphics_type = LayerSet::Layered(State::from(line_vec[1].clone()), Vec::new());
                    } else {
                        index_err!(i_b_line, len, 2, raw_buffer, Creature);
                    }
                },
                _ => {
                    match graphics_type {
                        LayerSet::Empty => {},//do nothing, haven't entered creature somehow
                        LayerSet::Simple(_) => {
                            if len >= 4 {
                                let mut reduced_line = line_vec.clone();
                                reduced_line.retain(|l| l.ne("AS_IS"));
                                let reduced_len = reduced_line.len();

                                if reduced_len == 4 || reduced_len == 5 {
                                    creature.graphics_type.push(LayerSet::Simple(vec![SimpleLayer{
                                        state: State::from(line_vec[0].clone()),
                                        tile: reduced_line[1].clone(),
                                        coords:
                                            [block_err_wrap!(reduced_line[2].parse(), i_b_line, raw_buffer, 2..=2),
                                            block_err_wrap!(reduced_line[3].parse(), i_b_line, raw_buffer, 3..=3)],
                                        large_coords: None,
                                        sub_state: if reduced_line.get(4).is_some() {
                                            Some(State::from(reduced_line[4].clone()))
                                        } else {None},
                                    }]));
                                } else if reduced_len == 7 || reduced_len == 8 {
                                    let (x,y) = 
                                        (block_err_wrap!(line_vec[3].parse::<u32>(), i_b_line, raw_buffer, 3..=3),
                                        block_err_wrap!(line_vec[4].parse::<u32>(), i_b_line, raw_buffer, 4..=4));
                                    let (x_l,y_l) = 
                                        (block_err_wrap!(line_vec[5].parse::<u32>(), i_b_line, raw_buffer, 5..=5),
                                        block_err_wrap!(line_vec[6].parse::<u32>(), i_b_line, raw_buffer, 6..=6));
                                    creature.graphics_type.push(LayerSet::Simple(vec![SimpleLayer{
                                        state: State::from(line_vec[0].clone()),
                                        tile: reduced_line[1].clone(),
                                        coords: [x, y],
                                        large_coords: Some([x_l.abs_diff(x), y_l.abs_diff(y)]),
                                        sub_state: if reduced_line.get(7).is_some() {
                                            Some(State::from(reduced_line[7].clone()))
                                        } else {None},
                                    }]));
                                } else if reduced_line.contains(&"LARGE_IMAGE".to_string()) {
                                    index_err!(i_b_line, len, 7, raw_buffer, Creature);
                                } else {
                                    index_err!(i_b_line, len, 4, raw_buffer, Creature);
                                }
                            } else {
                                index_err!(i_b_line, len, 4, raw_buffer, Creature);
                            }
                        },
                        LayerSet::Statue(_) => {
                            if len >= 6 {
                                let (x,y) = 
                                    (block_err_wrap!(line_vec[2].parse::<u32>(), i_b_line, raw_buffer, 2..=2),
                                    block_err_wrap!(line_vec[3].parse::<u32>(), i_b_line, raw_buffer, 3..=3));
                                let (x_l,y_l) = 
                                    (block_err_wrap!(line_vec[4].parse::<u32>(), i_b_line, raw_buffer, 4..=4),
                                    block_err_wrap!(line_vec[5].parse::<u32>(), i_b_line, raw_buffer, 5..=5));
                                creature.graphics_type.push(LayerSet::Statue(vec![SimpleLayer{
                                    state: State::from(line_vec[0].clone()),
                                    tile: line_vec[1].clone(),
                                    coords: [x, y],
                                    large_coords: Some([x_l.abs_diff(x), y_l.abs_diff(y)]),
                                    sub_state: caste,
                                }]));
                            } else {
                                index_err!(i_b_line, len, 6, raw_buffer, Creature);
                            }
                            break;
                        },
                        LayerSet::Layered(_, _) => {
                            ls_buffer.push(line_vec.clone());
                        },
                    }
                },
            }
        }

        let i_b_line = buffer.len() - 1;
        ls_push!(creature, graphics_type, ls_buffer, i_b_line, raw_buffer);

        Ok(creature)
    }

    fn import_layer_set(state: State, ls_buffer: Vec<Vec<String>>, raw_buffer: Vec<String>) -> Result<LayerSet> {
        let mut layer_groups = Vec::new();
        let mut lg_buffer = Vec::new();

        for (i_b_line, line_vec) in ls_buffer.iter().enumerate() {
            match line_vec[0].as_str() {
                "LAYER_SET" => {},//do nothing
                "LAYER_GROUP" => {//($layer_groups:ident, $lg_buffer:ident, $i_b_line:ident, $raw_buffer:ident)
                    lg_push!(layer_groups, lg_buffer, i_b_line, raw_buffer);
                    lg_buffer.push(line_vec.clone());
                },
                "END_LAYER_GROUP" => {
                    lg_push!(layer_groups, lg_buffer, i_b_line, raw_buffer);
                    lg_buffer.push(line_vec.clone());
                },
                _ => {
                    lg_buffer.push(line_vec.clone());
                }
            }
        }

        let i_b_line = ls_buffer.len() - 1;
        lg_push!(layer_groups, lg_buffer, i_b_line, raw_buffer);

        Self::rename_layer_groups(state.clone(), &mut layer_groups);
        
        Ok(LayerSet::Layered(state, layer_groups))
    }

    fn import_layer_group(lg_buffer: Vec<Vec<String>>, raw_buffer: Vec<String>) -> Result<LayerGroup> {
        let name = String::new();
        let mut layers = Vec::new();
        let mut l_buffer = Vec::new();

        for (i_b_line, line_vec) in lg_buffer.iter().enumerate() {
            match line_vec[0].as_str() {
                "LAYER_GROUP" | "END_LAYER_GROUP" => {},
                "LAYER" => {
                    if !l_buffer.is_empty() {
                        layers.push(
                            block_err_wrap!(Self::import_layer(l_buffer.clone(), raw_buffer.clone()), i_b_line, raw_buffer, 0..=0)
                        );
                        l_buffer.clear();
                    }
                    l_buffer.push(line_vec.clone());
                },
                _ => {
                    l_buffer.push(line_vec.clone());
                },
            }
        }
        
        let i_b_line = lg_buffer.len() - 1;
        if !l_buffer.is_empty() {
            layers.push(
                block_err_wrap!(Self::import_layer(l_buffer.clone(), raw_buffer.clone()), i_b_line, raw_buffer, 0..=0)
            );
            l_buffer.clear();
        }

        Ok(LayerGroup {name, layers})
    }

    fn import_layer(l_buffer: Vec<Vec<String>>, raw_buffer: Vec<String>) -> Result<Layer> {
        let mut layer = Layer::new();
        let mut conditions = Vec::new();
        let mut i_start = 0;

        for (i_b_line, line_vec) in l_buffer.iter().enumerate() {
            let len = line_vec.len();
            match line_vec[0].as_str() {
                "LAYER" => {
                    if len >= 5 {
                        let mut reduced_line = line_vec.clone();
                        reduced_line.retain(|l| l.ne("AS_IS"));
                        let reduced_len = reduced_line.len();

                        if reduced_len == 5 {
                            layer = Layer {
                                name: reduced_line[1].clone(),
                                tile: reduced_line[2].clone(),
                                coords:
                                    [block_err_wrap!(reduced_line[3].parse(), i_b_line, raw_buffer, 3..=3),
                                    block_err_wrap!(reduced_line[4].parse(), i_b_line, raw_buffer, 4..=4)],
                                large_coords: None,
                                conditions: conditions.clone(),
                            };
                        } else if reduced_len == 8 {
                            let (x,y) = 
                                (block_err_wrap!(line_vec[4].parse::<u32>(), i_b_line, raw_buffer, 4..=4),
                                block_err_wrap!(line_vec[5].parse::<u32>(), i_b_line, raw_buffer, 5..=5));
                            let (x_l,y_l) = 
                                (block_err_wrap!(line_vec[6].parse::<u32>(), i_b_line, raw_buffer, 6..=6),
                                block_err_wrap!(line_vec[7].parse::<u32>(), i_b_line, raw_buffer, 7..=7));
                            layer = Layer {
                                name: reduced_line[1].clone(),
                                tile: reduced_line[2].clone(),
                                coords: [x, y],
                                large_coords: Some([x_l.abs_diff(x), y_l.abs_diff(y)]),
                                conditions: conditions.clone(),
                            };
                        } else if reduced_line.contains(&"LARGE_IMAGE".to_string()) {
                            index_err!(i_b_line, len, 8, raw_buffer, Layer);
                        } else {
                            index_err!(i_b_line, len, 5, raw_buffer, Layer);
                        }
                    } else {
                        index_err!(i_b_line, len, 5, raw_buffer, Layer);
                    }
                },
                _ => {
                    conditions.push(block_err_wrap!(Condition::from(line_vec.clone()), i_b_line, raw_buffer, 0..=0));//todo fix index
                },
            }
        }

        layer.conditions = conditions;

        Ok(layer)
    }

    fn rename_layer_groups(state: State, layer_groups: &mut Vec<LayerGroup>) {
        for lg in layer_groups.iter_mut() {
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

    pub fn display(&self, path: &path::PathBuf) -> Result<()> {
        fs::DirBuilder::new()
            .recursive(true)
            .create(path.join("graphics").join("images"))?;

        for tile_page in self.tile_pages.iter() {
            tile_page.display(&path)?;
        }

        for creature_file in self.creature_files.iter() {
            creature_file.display(&path)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct CreatureFile {
    pub name: String,             //graphcics_creatures_ name
    pub creatures: Vec<Creature>, //set of creatures/creature graphics types to group in one file
}
impl CreatureFile {
    pub fn new() -> CreatureFile {
        CreatureFile {
            name: String::new(),
            creatures: vec![Creature::new()],
        }
    }

    fn display(&self, path: &path::PathBuf) -> Result<()> {
        let creature_file = fs::File::create(
            path
            .join("graphics")
            .join(format!("graphics_creatures_{}.txt",
            self.name.with_boundaries(&[Boundary::Space])
            .to_case(Case::Snake)))
        )?;

        let mut creature_writer = io::LineWriter::new(creature_file);
        
        creature_writer.write_all(format!(
            "graphics_creatures_{}\n\n[OBJECT:GRAPHICS]\n\n",
            self.name.with_boundaries(&[Boundary::Space])
            .to_case(Case::Snake))
            .as_bytes()
        )?;

        for creature in self.creatures.iter() {
            creature_writer.write_all(creature.display().as_bytes())?;
        }
        
        creature_writer.flush()?;

        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum State {
    #[default]
    Empty,
    Default,
    Child,
    Baby,
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
    pub fn name(&self) -> String {
        match self {
            Self::Default => "DEFAULT".to_string(),
            Self::Child => "CHILD".to_string(),
            Self::Baby => "BABY".to_string(),
            Self::Animated => "ANIMATED".to_string(),
            Self::Corpse => "CORPSE".to_string(),
            Self::ListIcon => "LIST_ICON".to_string(),
            Self::TrainedHunter => "TRAINED_HUNTER".to_string(),
            Self::TrainedWar => "TRAINED_WAR".to_string(),
            Self::Skeleton => "SKELETON".to_string(),
            Self::SkeletonWithSkull => "SKELETON_WITH_SKULL".to_string(),
            Self::Custom(name) => {
                name.with_boundaries(&[Boundary::Space])
                    .to_case(Case::UpperSnake)
                    .to_string()
            },
            Self::Empty => "(empty)".to_string(),
        }
    }

    fn iterator() -> std::slice::Iter<'static, Self> {
        static STATES: [State; 10] = [
            State::Default,
            State::Child,
            State::Baby,
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
            "DEFAULT" => State::Default,
            "CHILD" => State::Child,
            "BABY" => State::Baby,
            "ANIMATED" => State::Animated,
            "CORPSE" => State::Corpse,
            "LIST_ICON" => State::ListIcon,
            "TRAINED_HUNTER" => State::TrainedHunter,
            "TRAINED_WAR" => State::TrainedWar,
            "SKELETON" => State::Skeleton,
            "SKELETON_WITH_SKULL" => State::SkeletonWithSkull,
            other => { State::Custom(other.to_uppercase().to_string()) }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum LayerSet {
    #[default]
    Empty,
    Simple(Vec<SimpleLayer>),
    Layered(State, Vec<LayerGroup>),
    Statue(Vec<SimpleLayer>),
}
impl LayerSet {
    pub fn layer_set_menu(&mut self, ui: &mut Ui) {
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

    fn display(&self) -> String {
        let mut out = String::new();

        match self {
            LayerSet::Simple(simple_layers) => {
                for simple_layer in simple_layers {
                    out.push_str(&simple_layer.display());
                }

                out.push_str("\n");
            },
            LayerSet::Statue(simple_layers) => {
                for simple_layer in simple_layers {
                    out.push_str(&simple_layer.display_statue());
                }

                out.push_str("\n");
            },
            LayerSet::Layered(state, layer_groups) => {
                out.push_str(&format!("\t[LAYER_SET:{}]\n", state.name()));

                for layer_group in layer_groups {
                    out.push_str(&layer_group.display());
                }
            },
            LayerSet::Empty => {}
        }

        out
    }
}

#[derive(Clone, Default, Debug)]
pub struct Creature {
    pub name: String,                 //file name of creature_graphics_file_name.txt
    pub graphics_type: Vec<LayerSet>, //which type of graphics (layered, simple, caste, fb)
}
impl Creature {
    pub fn new() -> Creature {
        Creature {
            name: String::from("(new)"),
            graphics_type: Vec::new(),
        }
    }

    pub fn creature_menu(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.text_edit_singleline(&mut self.name);

        ui.add_space(PADDING);
        if ui.button("Add simple layer").clicked() {
            if self.graphics_type.iter().any(|ls| match ls { LayerSet::Layered(..) => true, _ => false}) {
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

    fn sort_layer_sets(layer_sets: &Vec<LayerSet>) -> Vec<LayerSet> {
        let mut statues = layer_sets.clone();
        let mut simples = layer_sets.clone();
        let mut layereds = layer_sets.clone();

        statues.retain(|ls| if let LayerSet::Statue(..) = ls {true} else {false});
        simples.retain(|ls| if let LayerSet::Simple(..) = ls {true} else {false});
        layereds.retain(|ls| if let LayerSet::Layered(..) = ls {true} else {false});
        
        [statues, simples, layereds].concat()
    }

    fn display(&self) -> String {
        let mut out = String::new();

        let mut layer_sets = Self::sort_layer_sets(&self.graphics_type);

        for layer_set in layer_sets.iter_mut() {//test
            if let LayerSet::Statue(simple_layers) = layer_set {
                if !out.contains("[STATUE_CREATURE_GRAPHICS ") {
                    out.push_str(&format!(
                        "[STATUE_CREATURE_GRAPHICS:{}]\n",
                        self.name.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                    ));
                } else if simple_layers.len() > 1 {
                    simple_layers.drain(1..);
                }
            } else if let LayerSet::Empty = layer_set {
                
            } else {
                if !out.contains("[CREATURE_GRAPHICS") {
                    out.push_str(&format!(
                        "[CREATURE_GRAPHICS:{}]\n",
                        self.name.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                    ));
                }
            }
            
            out.push_str(&layer_set.display());
        }

        out
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SimpleLayer {
    pub state: State,
    pub tile: String,
    pub coords: [u32; 2],
    pub large_coords: Option<[u32; 2]>,
    pub sub_state: Option<State>,
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

    pub fn new() -> SimpleLayer {
        SimpleLayer {
            state: State::Default,
            tile: String::new(),
            coords: [0, 0],
            large_coords: None,
            sub_state: None,
        }
    }

    pub fn layer_menu(&mut self, ui: &mut Ui, tile_info: Vec<(String, [u32;2])>) {
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
            ui.hyperlink_to("Custom states that may work.", "https://dwarffortresswiki.org/index.php/Graphics_token#Conditions");
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
            ui.hyperlink_to("Custom states that may work.", "https://dwarffortresswiki.org/index.php/Graphics_token#Conditions");
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
            ui.add(egui::Slider::new(x1, 0..=max_coords[0].checked_sub(*x2)
                .unwrap_or_default()).prefix("Tile X: "));
            ui.add(egui::Slider::new(y1, 0..=max_coords[1].checked_sub(*y2)
                .unwrap_or_default()).prefix("Tile Y: "));

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
            ui.add(egui::Label::new(self.display()).wrap(false));
        });
    }

    pub fn statue_layer_menu(&mut self, ui: &mut Ui, tile_info: Vec<(String, [u32;2])>) {
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
            ui.hyperlink_to("Custom states that may work.", "https://dwarffortresswiki.org/index.php/Graphics_token#Conditions");
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

        ui.add(egui::Slider::new(x1, 0..=max_coords[0].checked_sub(*x2)
            .unwrap_or_default()).prefix("Tile X: "));
        ui.add(egui::Slider::new(y1, 0..=max_coords[1].checked_sub(*y2)
            .unwrap_or_default()).prefix("Tile Y: "));

        ui.add(egui::Slider::new(x2, 0..=2).prefix("X + "));
        ui.add(egui::Slider::new(y2, 0..=1).prefix("Y + "));

        ui.add_space(PADDING);
        ui.label("Preview:");
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.add(egui::Label::new(self.display()).wrap(false));
        });
    }

    fn display(&self) -> String {
        if let Some([x2, y2]) = self.large_coords {
            if let Some(sub_state) = &self.sub_state {
                format!(
                    "\t\t[{}:{}:LARGE_IMAGE:{}:{}:{}:{}:AS_IS:{}]\n",
                    self.state.name(),
                    self.tile.with_boundaries(&[Boundary::Space])
                        .to_case(Case::UpperSnake)
                        .to_string(),
                    self.coords[0],
                    self.coords[1],
                    self.coords[0] + x2,
                    self.coords[1] + y2,
                    sub_state.name(),
                )
            } else {
                format!(
                    "\t[{}:{}:LARGE_IMAGE:{}:{}:{}:{}:AS_IS]\n",
                    self.state.name(),
                    self.tile.with_boundaries(&[Boundary::Space])
                        .to_case(Case::UpperSnake)
                        .to_string(),
                    self.coords[0],
                    self.coords[1],
                    self.coords[0] + x2,
                    self.coords[1] + y2,
                )
            }
        } else {
            if let Some(sub_state) = &self.sub_state {
                format!(
                    "\t\t[{}:{}:{}:{}:AS_IS:{}]\n",
                    self.state.name(),
                    self.tile.with_boundaries(&[Boundary::Space])
                        .to_case(Case::UpperSnake)
                        .to_string(),
                    self.coords[0],
                    self.coords[1],
                    sub_state.name(),
                )
            } else {
                format!(
                    "\t[{}:{}:{}:{}:AS_IS]\n",
                    self.state.name(),
                    self.tile.with_boundaries(&[Boundary::Space])
                        .to_case(Case::UpperSnake)
                        .to_string(),
                    self.coords[0],
                    self.coords[1],
                )
            }
        }
    }

    fn display_statue(&self) -> String {
        if let Some([x2, y2]) = self.large_coords {
            format!(
                "\t[{}:{}:{}:{}:{}:{}]\n",
                self.state.name(),
                self.tile.with_boundaries(&[Boundary::Space])
                    .to_case(Case::UpperSnake)
                    .to_string(),
                self.coords[0],
                self.coords[1],
                self.coords[0] + x2,
                self.coords[1] + y2,
            )
        } else {String::new()}
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layer {
    pub name: String,                     //LAYER_NAME for patterning
    pub conditions: Vec<Condition>,       //Set of condition(s) that layer displays in
    pub tile: String,                     //TILE_NAME of image
    pub coords: [u32; 2],               //x,y coordinates of layer on image in tiles
    pub large_coords: Option<[u32; 2]>, //(optional) x2,y2 coordinates of bottom right corner of layer in tiles
}
impl Layer {
    pub fn new() -> Layer {
        Layer {
            name: "(new)".to_string(),
            conditions: vec![Condition::default()],
            tile: String::new(),
            coords: [0, 0],
            large_coords: None,
        }
    }

    pub fn layer_menu(&mut self, ui: &mut Ui, tile_info: Vec<(String, [u32; 2])>) {
        
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
                ui[0].add(egui::Slider::new(x1, 0..=max_coords[0].checked_sub(*x2)
                    .unwrap_or_default()).prefix("Tile X: "));
                ui[0].add(egui::Slider::new(y1, 0..=max_coords[1].checked_sub(*y2)
                    .unwrap_or_default()).prefix("Tile Y: "));

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
                    layer.display())
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
                        ui.group(|ui| {
                            condition.condition_menu(ui, tile_info.clone());
                            ui.add_space(PADDING);
                            if ui.button("Remove Condition").clicked() {
                                delete = Some(i_cond);
                            }
                        });
                        
                        ui.add_space(PADDING);
                    });
                }
            });

            if let Some(i_cond) = delete {
                conditions.remove(i_cond);//checked
            }
        });
    }

    fn display(&self) -> String {
        let mut out = String::new();

        if let Some([x2, y2]) = self.large_coords {
            out.push_str(&format!(
                "\t\t\t[LAYER:{}:{}:LARGE_IMAGE:{}:{}:{}:{}:AS_IS]\n",
                self.name.with_boundaries(&[Boundary::Space])
                    .to_case(Case::UpperSnake)
                    .to_string(),
                self.tile.with_boundaries(&[Boundary::Space])
                    .to_case(Case::UpperSnake)
                    .to_string(),
                self.coords[0],
                self.coords[1],
                self.coords[0] + x2,
                self.coords[1] + y2,
            ));
        } else {
            out.push_str(&format!(
                "\t\t\t[LAYER:{}:{}:{}:{}:AS_IS]\n",
                self.name.with_boundaries(&[Boundary::Space])
                    .to_case(Case::UpperSnake)
                    .to_string(),
                self.tile.with_boundaries(&[Boundary::Space])
                    .to_case(Case::UpperSnake)
                    .to_string(),
                self.coords[0],
                self.coords[1],
            ));
        }

        for condition in self.conditions.iter() {
            out.push_str(&condition.display());
        }

        out
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayerGroup {
    pub name: String,       //internal layer group name
    pub layers: Vec<Layer>, //set of layers to display for creature
}
impl LayerGroup {
    pub fn new() -> LayerGroup {
        LayerGroup {
            name: "(new)".to_string(),
            layers: vec![Layer::new()],
        }
    }

    pub fn layer_group_menu(&mut self, ui: &mut Ui) {
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
            ui.add(egui::Label::new(self.display()).wrap(false));
        });
    }

    fn display(&self) -> String {
        let mut out = String::new();

        out.push_str(&format!(
            "\t\t[LAYER_GROUP] ---{}---\n",
            self.name
            .with_boundaries(&[Boundary::Space])
            .to_case(Case::UpperSnake)
            .to_string()
        ));
        for layer in self.layers.iter() {
            out.push_str(&layer.display());
        }
        out.push_str("\n");

        out
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Metal {
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
            Metal::Custom(metal) => {
                metal.with_boundaries(&[Boundary::Space])
                    .to_case(Case::UpperSnake)
                    .to_string()
            },
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
pub enum MaterialFlag {
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
pub enum ItemType {
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

    fn from(strings: Vec<String>) -> Result<(ItemType, Vec<String>)> {
        let len = strings.len();
        match strings[0].as_str() {
            "BY_CATEGORY" => {
                if len > 3 {
                    Ok(
                        (ItemType::ByCategory(strings[1].clone(),
                        Equipment::from(strings[2].clone())),
                        strings[3..].to_vec())
                    )
                } else {
                    // return Err(DFGHError::ImportConditionError(strings.join(":")))
                    return Err(DFGHError::None)//todo fix
                }
            },
            "BY_TOKEN" => {
                if len > 3 {
                    Ok(
                        (ItemType::ByToken(strings[1].clone(),
                        Equipment::from(strings[2].clone())),
                        strings[3..].to_vec())
                    )
                } else {
                    // return Err(DFGHError::ImportConditionError(strings.join(":")))
                    return Err(DFGHError::None)//todo fix
                }
            },
            "ANY_HELD" => {
                if len > 2 {
                    Ok(
                        (ItemType::AnyHeld(Equipment::from(strings[1].clone())),
                        strings[2..].to_vec())
                    )
                } else {
                    // return Err(DFGHError::ImportConditionError(strings.join(":")))
                    return Err(DFGHError::None)//todo fix
                }
            },
            "WIELD" => {
                if len > 2 {
                    Ok(
                        (ItemType::Wield(Equipment::from(strings[1].clone())),
                        strings[2..].to_vec())
                    )
                } else {
                    // return Err(DFGHError::ImportConditionError(strings.join(":")))
                    return Err(DFGHError::None)//todo fix
                }
            },
            _ => {Ok((ItemType::None, strings))}
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Equipment {
    #[default]
    None,
    Armor,
    Helm,
    Gloves,
    Shoes,
    Pants,
    Shield,
    Weapon,
    Tool,
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
            Equipment::Tool => "TOOL".to_string(),
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
            "TOOL" => Equipment::Tool,
            "ANY" => Equipment::Any,
            _ => Equipment::None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Profession {
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
            Profession::Custom(prof) => {
                prof.with_boundaries(&[Boundary::Space])
                    .to_case(Case::UpperSnake)
                    .to_string()
            },
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
            "FISHERY_WORKER" => Profession::FisheryWorker,
            "MERCHANT" => Profession::Merchant,
            "NONE" => Profession::None,
            prof=> Profession::Custom(prof.to_string()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Condition {
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
    pub fn name(&self) -> String {
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
            Self::Custom(string) => {
                string.with_boundaries(&[Boundary::Space])
                    .to_case(Case::UpperSnake)
                    .to_string()
            },
        }
    }

    fn from(mut line_vec: Vec<String>) -> Result<Condition> {
        let len = line_vec.len();
        if len > 0 {
            match line_vec[0].as_str() {
                "(default)" => Ok(Condition::Default),
                "CONDITION_ITEM_WORN" => {
                    let (item_type, items) = ItemType::from(line_vec[1..].to_vec())?;
                    Ok(Condition::ItemWorn(item_type, items))
                },
                "SHUT_OFF_IF_ITEM_PRESENT" => {
                    let (item_type, items) = ItemType::from(line_vec[1..].to_vec())?;
                    Ok(Condition::ShutOffIfItemPresent(item_type, items))
                },
                "CONDITION_DYE" => {
                    if len > 1 {
                        Ok(Condition::Dye(line_vec[1].clone()))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "CONDITION_NOT_DYED" => Ok(Condition::NotDyed),
                "CONDITION_MATERIAL_FLAG" => {
                    if len > 1 {
                        Ok(Condition::MaterialFlag(
                            line_vec[1..]
                                .iter()
                                .map(|flag| MaterialFlag::from(flag.clone()))
                                .collect()
                        ))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "CONDITION_MATERIAL_TYPE" => {
                    if len > 2 {
                        Ok(Condition::MaterialType(
                            Metal::from(line_vec[2].clone())
                        ))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "CONDITION_PROFESSION_CATEGORY" => {
                    if len > 1 {
                        Ok(Condition::ProfessionCategory(
                            line_vec[1..]
                                .iter()
                                .map(|prof| Profession::from(prof.clone()))
                                .collect()
                        ))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "CONDITION_RANDOM_PART_INDEX" => {
                    if len > 3 {
                        Ok(Condition::RandomPartIndex(
                            line_vec[1].clone(),
                            line_vec[2].parse()?,
                            line_vec[3].parse()?
                        ))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "CONDITION_HAUL_COUNT_MIN" => {
                    if len > 1 {
                        Ok(Condition::HaulCountMin(
                            line_vec[1].parse()?
                        ))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "CONDITION_HAUL_COUNT_MAX" => {
                    if len > 1 {
                        Ok(Condition::HaulCountMax(
                            line_vec[1].parse()?
                        ))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "CONDITION_CHILD" => Ok(Condition::Child),
                "CONDITION_NOT_CHILD" => Ok(Condition::NotChild),
                "CONDITION_CASTE" => {
                    if len > 1 {
                        Ok(Condition::Caste(
                            line_vec[1].clone()
                        ))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "CONDITION_GHOST" => Ok(Condition::Ghost),
                "CONDITION_SYN_CLASS" => {
                    if len > 1 {
                        Ok(Condition::SynClass(
                            line_vec.drain(1..).collect()
                        ))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "CONDITION_TISSUE_LAYER" => {
                    if len > 3 {
                        Ok(Condition::TissueLayer(
                            line_vec[2].clone(),
                            line_vec[3].clone(),
                        ))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "TISSUE_MIN_LENGTH" => {
                    if len > 0 {
                        Ok(Condition::TissueMinLength(
                            line_vec[1].parse()?
                        ))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "TISSUE_MAX_LENGTH" => {
                    if len > 1 {
                        Ok(Condition::TissueMaxLength(
                            line_vec[1].parse()?
                        ))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "TISSUE_MAY_HAVE_COLOR" => {
                    if len > 1 {
                        Ok(Condition::TissueMayHaveColor(
                            line_vec.drain(1..).collect()
                        ))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "TISSUE_MAY_HAVE_SHAPING" => {
                    if len > 1 {
                        Ok(Condition::TissueMayHaveShaping(
                            line_vec.drain(1..).collect()
                        ))
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                "TISSUE_NOT_SHAPED" => Ok(Condition::TissueNotShaped),
                "TISSUE_SWAP" => {
                    if len > 5 {
                        if line_vec[4].eq("LARGE_IMAGE") && len > 8 {
                            let c = [line_vec[5].parse()?,
                                line_vec[6].parse()?];
                            let l_c = [line_vec[7].parse()?,
                                line_vec[8].parse()?];
                            let large;
                            if (c[0] <= l_c[0]) && (c[1] <= l_c[1]) {
                                large = [l_c[0]-c[0], l_c[1]-c[1]]; 
                                Ok(Condition::TissueSwap(
                                    line_vec[1].clone(),
                                    line_vec[2].parse()?,
                                    line_vec[3].clone(),
                                    c,
                                    Some(large),
                                ))
                            } else {
                                // return Err(DFGHError::ImportConditionError(line_vec.join(":")))
                                return Err(DFGHError::None)//todo fix
                            }
                        } else {
                            Ok(Condition::TissueSwap(
                                line_vec[1].clone(),
                                line_vec[2].parse()?,
                                line_vec[3].clone(),
                                [line_vec[4].parse()?, 
                                line_vec[5].parse()?],
                                None,
                            ))
                        }
                    } else {
                        // Err(DFGHError::ImportConditionError(line_vec.join(":")))
                        return Err(DFGHError::None)//todo fix
                    }
                },
                other => Ok(Condition::Custom(other.to_string())),
            }
        } else {
            // Err(DFGHError::ImportConditionError(line_vec.join(":")))
            Err(DFGHError::None)//todo fix
        }
    }

    pub fn condition_menu(&mut self, ui: &mut Ui, tile_info: Vec<(String, [u32; 2])>) {
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

                        for item in items.iter_mut() {
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

                        for item in items.iter_mut() {
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

                        for item in items.iter_mut() {
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
                    ItemType::Wield(equipment) => {
                        ui.label("Item type: (WEAPON, TOOL, or ANY)");
                        egui::ComboBox::from_label("Item type")
                            .selected_text(&equipment.name())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(equipment, Equipment::Any, "Any");
                                ui.selectable_value(equipment, Equipment::Weapon, "Weapon");
                                ui.selectable_value(equipment, Equipment::Tool, "Tool");
                        });

                        if equipment == &Equipment::Any {
                            items.clear();
                        } else {
                            ui.label("Item: (e.g. ITEM_WEAPON_PICK)");
    
                            for item in items.iter_mut() {
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
                ui.hyperlink_to("Only METAL:<weapons metal> tokens are currently valid (v50.05).", "https://dwarffortresswiki.org/index.php/Graphics_token#CONDITION_MATERIAL_TYPE");
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
                    ui.add(egui::Slider::new(x1, 0..=max_coords[0].checked_sub(*x2)
                        .unwrap_or_default()).prefix("Tile X: "));
                    ui.add(egui::Slider::new(y1, 0..=max_coords[1].checked_sub(*y2)
                        .unwrap_or_default()).prefix("Tile Y: "));

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

        ui.add_space(PADDING);
        ui.label(self.display());
    }

    fn display(&self) -> String {
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
                            equipment.name()
                        ));
                    },
                    ItemType::Wield(equipment) => {
                        out.push_str(&format!(
                            "WIELD:{}",
                            equipment.name()
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
                out = format!("\t\t\t\t[SHUT_OFF_IF_ITEM_PRESENT:");
                match item_type {
                    ItemType::ByCategory(category, equipment) => {
                        out.push_str(&format!("BY_CATEGORY:{}:{}",
                            category.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                            equipment.name()
                        ));
                    },
                    ItemType::ByToken(token, equipment) => {
                        out.push_str(&format!("BY_TOKEN:{}:{}",
                            token.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                            equipment.name()
                        ));
                    },
                    ItemType::AnyHeld(equipment) => {
                        out.push_str(&format!(
                            "ANY_HELD:{}",
                            equipment.name()
                        ));
                    },
                    ItemType::Wield(equipment) => {
                        out.push_str(&format!(
                            "WIELD:{}",
                            equipment.name()
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
                        flag.name()
                    ));
                }
                out.push_str("]\n");
            },
            Condition::MaterialType(metal) => {
                out = format!(
                    "\t\t\t\t[CONDITION_MATERIAL_TYPE:METAL:{}]\n",
                    metal.name()
                );
            },
            Condition::ProfessionCategory(professions) => {
                out = "\t\t\t\t[CONDITION_PROFESSION_CATEGORY".to_string();
                for profession in professions {
                    out.push_str(&format!(
                        ":{}",
                        profession.name()
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
                    "\t\t\t\t\t[TISSUE_SWAP:{}:{}:{}:",
                    app_mod.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                    amount,
                    tile.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
                );

                if let Some([x2,y2]) = large_coords {
                    out.push_str(&format!(
                        "LARGE_IMAGE:{}:{}:{}:{}]\n",
                        x1,
                        y2,
                        x1 + x2,
                        y1 + y2
                    ));
                } else {
                    out.push_str(&format!(
                        "{}:{}]\n",
                        x1,
                        y1,
                    ));
                }
            },
            Condition::Custom(string) => {
                out = format!("\t\t\t\t[{0}]\n",
                    string.clone().with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake)
                );
            },
        }

        out
    }
}

#[derive(Clone, Default, Debug)]
pub struct TilePage {
    pub name: String,     //file name of tile_set_file_name.txt
    pub tiles: Vec<Tile>, //set of tiles defined in this file
}
impl TilePage {
    pub fn new() -> TilePage {
        TilePage {
            name: String::from("(new)"),
            tiles: vec![Tile::new()],
        }
    }

    fn display(&self, path: &path::PathBuf) -> Result<()> {
        let tile_page_file = fs::File::create(
            path
            .join("graphics")
            .join(format!("tile_page_{}.txt",
            self.name.clone()
            .with_boundaries(&[Boundary::Space])
            .to_case(Case::Snake)))
        )?;

        let mut tile_page_writer = io::LineWriter::new(tile_page_file);
        
        tile_page_writer.write_all(format!(
            "tile_page_{}\n\n[OBJECT:TILE_PAGE]\n\n",
            self.name
            .with_boundaries(&[Boundary::Space])
            .to_case(Case::Snake)
            ).as_bytes()
        )?;

        for tile in self.tiles.iter() {
            tile_page_writer.write_all(tile.display()
                .as_bytes())?;
        }
        
        tile_page_writer.flush()?;

        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tile {
    pub name: String,           //all-caps NAME of tile
    pub filename: String,       //file path of image.png
    pub image_size: [u32; 2], //size of image in pixels
    pub tile_size: [u32; 2],  //size of tile in pixels
}
impl Tile {
    pub fn new() -> Tile {
        Tile {
            name: "(new)".to_string(),
            filename: String::new(),
            image_size: [0, 0],
            tile_size: [32, 32],
        }
    }

    pub fn tile_menu(&mut self, ui: &mut Ui) {
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
            ui.add(egui::Label::new(self.display()).wrap(false));
        });
    }

    fn display(&self) -> String {
        format!(
            "[TILE_PAGE:{}]\n\t[FILE:images/{}.png]\n\t[TILE_DIM:{}:{}]\n\t[PAGE_DIM_PIXELS:{}:{}]\n\n",
            self.name.with_boundaries(&[Boundary::Space]).to_case(Case::UpperSnake),
            self.filename.with_boundaries(&[Boundary::Space]).to_case(Case::Snake),
            self.tile_size[0],
            self.tile_size[1],
            self.image_size[0],
            self.image_size[1]
        )
    }
}