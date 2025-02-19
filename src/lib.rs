//! Library for getting and matching system fonts with
//! minimal dependencies
//!
//! # Usage
//!
//! ```rust
//! use dafont::{FcFontCache, FcPattern};
//!
//! fn main() {
//!
//!     let cache = FcFontCache::build();
//!     let results = cache.query(&FcPattern {
//!         name: Some(String::from("Arial")),
//!         .. Default::default()
//!     });
//!
//!     println!("font results: {:?}", results);
//! }
//! ```

#![allow(non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "parsing")]
extern crate allsorts;
#[cfg(all(not(target_family = "wasm"), feature = "std"))]
extern crate mmapio;
extern crate xmlparser;

extern crate alloc;
extern crate core;

use alloc::borrow::ToOwned;
use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::path::PathBuf;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[repr(C)]
pub enum PatternMatch {
    True,
    False,
    DontCare,
}

impl PatternMatch {
    fn needs_to_match(&self) -> bool {
        matches!(self, PatternMatch::True | PatternMatch::False)
    }
}

impl Default for PatternMatch {
    fn default() -> Self {
        PatternMatch::DontCare
    }
}

#[derive(Debug, Default, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[repr(C)]
pub struct FcPattern {
    // font name
    pub name: Option<String>,
    // family name
    pub family: Option<String>,
    // "italic" property
    pub italic: PatternMatch,
    // "oblique" property
    pub oblique: PatternMatch,
    // "bold" property
    pub bold: PatternMatch,
    // "monospace" property
    pub monospace: PatternMatch,
    // "condensed" property
    pub condensed: PatternMatch,
    // font weight
    pub weight: usize,
    // start..end unicode range
    pub unicode_range: [usize; 2],
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[repr(C)]
pub struct FcFontPath {
    pub path: String,
    pub font_index: usize,
}

/// Represent an in-memory font file
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
#[repr(C)]
pub struct FcFont {
    pub bytes: Vec<u8>,
    pub font_index: usize,
}

#[derive(Debug, Default, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct FcFontCache {
    map: BTreeMap<FcPattern, FcFontPath>,
}

impl FcFontCache {
    /// Adds in-memory font files (`path` will be base64 encoded)
    pub fn with_memory_fonts(&mut self, f: &[(FcPattern, FcFont)]) -> &mut Self {
        use base64::{engine::general_purpose::URL_SAFE, Engine as _};
        self.map.extend(f.iter().map(|(k, v)| {
            (
                k.clone(),
                FcFontPath {
                    path: {
                        let mut s = String::from("base64:");
                        s.push_str(&URL_SAFE.encode(&v.bytes));
                        s
                    },
                    font_index: v.font_index,
                },
            )
        }));
        self
    }

    /// Builds a new font cache
    #[cfg(not(all(feature = "std", feature = "parsing")))]
    pub fn build() -> Self {
        Self::default()
    }

    /// Builds a new font cache from all fonts discovered on the system
    ///
    /// NOTE: Performance-intensive, should only be called on startup!
    #[cfg(all(feature = "std", feature = "parsing"))]
    pub fn build() -> Self {
        #[cfg(target_os = "linux")]
        {
            FcFontCache {
                map: FcScanDirectories()
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
            }
        }

        #[cfg(target_os = "windows")]
        {
            // `~` isn't actually valid on Windows, but it will be converted by `process_path`
            let font_dirs = vec![
                (None, "C:\\Windows\\Fonts\\".to_owned()),
                (
                    None,
                    "~\\AppData\\Local\\Microsoft\\Windows\\Fonts\\".to_owned(),
                ),
            ];
            FcFontCache {
                map: FcScanDirectoriesInner(&font_dirs).into_iter().collect(),
            }
        }

        #[cfg(target_os = "macos")]
        {
            let font_dirs = vec![
                (None, "~/Library/Fonts".to_owned()),
                (None, "/System/Library/Fonts".to_owned()),
                (None, "/Library/Fonts".to_owned()),
            ];
            FcFontCache {
                map: FcScanDirectoriesInner(&font_dirs).into_iter().collect(),
            }
        }

        #[cfg(target_family = "wasm")]
        {
            Self::default()
        }
    }

    /// Returns the list of fonts and font patterns
    pub fn list(&self) -> &BTreeMap<FcPattern, FcFontPath> {
        &self.map
    }

    fn query_matches_internal(k: &FcPattern, pattern: &FcPattern) -> bool {
        let name_needs_to_match = pattern.name.is_some();
        let family_needs_to_match = pattern.family.is_some();

        let italic_needs_to_match = pattern.italic.needs_to_match();
        let oblique_needs_to_match = pattern.oblique.needs_to_match();
        let bold_needs_to_match = pattern.bold.needs_to_match();
        let monospace_needs_to_match = pattern.monospace.needs_to_match();

        let name_matches = k.name == pattern.name;
        let family_matches = k.family == pattern.family;
        let italic_matches = k.italic == pattern.italic;
        let oblique_matches = k.oblique == pattern.oblique;
        let bold_matches = k.bold == pattern.bold;
        let monospace_matches = k.monospace == pattern.monospace;

        if name_needs_to_match && !name_matches {
            return false;
        }

        if family_needs_to_match && !family_matches {
            return false;
        }

        if name_needs_to_match && !name_matches {
            return false;
        }

        if family_needs_to_match && !family_matches {
            return false;
        }

        if italic_needs_to_match && !italic_matches {
            return false;
        }

        if oblique_needs_to_match && !oblique_matches {
            return false;
        }

        if bold_needs_to_match && !bold_matches {
            return false;
        }

        if monospace_needs_to_match && !monospace_matches {
            return false;
        }

        true
    }

    /// Queries a font from the in-memory `font -> file` mapping, returns all matching fonts
    pub fn query_all(&self, pattern: &FcPattern) -> Vec<&FcFontPath> {
        self.map
            .iter() // TODO: par_iter!
            .filter(|(k, _)| Self::query_matches_internal(k, pattern))
            .map(|(_, v)| v)
            .collect()
    }

    /// Queries a font from the in-memory `font -> file` mapping, returns the first found font (early return)
    pub fn query(&self, pattern: &FcPattern) -> Option<&FcFontPath> {
        self.map
            .iter() // TODO: par_iter!
            .find(|(k, _)| Self::query_matches_internal(k, pattern))
            .map(|(_, v)| v)
    }
}

#[cfg(feature = "std")]
/// Takes a path & prefix and resolves them to a usable path, or `None` if they're unsupported/unavailable.
///
/// Behaviour is based on: https://www.freedesktop.org/software/fontconfig/fontconfig-user.html
fn process_path(
    prefix: &Option<String>,
    mut path: PathBuf,
    is_include_path: bool,
) -> Option<PathBuf> {
    use std::env::var;

    const HOME_SHORTCUT: &str = "~";
    const CWD_PATH: &str = ".";

    const HOME_ENV_VAR: &str = "HOME";
    const XDG_CONFIG_HOME_ENV_VAR: &str = "XDG_CONFIG_HOME";
    const XDG_CONFIG_HOME_DEFAULT_PATH_SUFFIX: &str = ".config";
    const XDG_DATA_HOME_ENV_VAR: &str = "XDG_DATA_HOME";
    const XDG_DATA_HOME_DEFAULT_PATH_SUFFIX: &str = ".local/share";

    const PREFIX_CWD: &str = "cwd";
    const PREFIX_DEFAULT: &str = "default";
    const PREFIX_XDG: &str = "xdg";

    // These three could, in theory, be cached, but the work required to do so outweighs the minor benefits
    fn get_home_value() -> Option<PathBuf> {
        var(HOME_ENV_VAR).ok().map(PathBuf::from)
    }
    fn get_xdg_config_home_value() -> Option<PathBuf> {
        var(XDG_CONFIG_HOME_ENV_VAR)
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                get_home_value()
                    .map(|home_path| home_path.join(XDG_CONFIG_HOME_DEFAULT_PATH_SUFFIX))
            })
    }
    fn get_xdg_data_home_value() -> Option<PathBuf> {
        var(XDG_DATA_HOME_ENV_VAR)
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                get_home_value().map(|home_path| home_path.join(XDG_DATA_HOME_DEFAULT_PATH_SUFFIX))
            })
    }

    // Resolve the tilde character in the path, if present
    if path.starts_with(HOME_SHORTCUT) {
        if let Some(home_path) = get_home_value() {
            path = home_path.join(
                path.strip_prefix(HOME_SHORTCUT)
                    .expect("already checked that it starts with the prefix"),
            );
        } else {
            return None;
        }
    }

    // Resolve prefix values
    match prefix {
        Some(prefix) => match prefix.as_str() {
            PREFIX_CWD | PREFIX_DEFAULT => {
                let mut new_path = PathBuf::from(CWD_PATH);
                new_path.push(path);

                Some(new_path)
            }
            PREFIX_XDG => {
                if is_include_path {
                    get_xdg_config_home_value()
                        .map(|xdg_config_home_path| xdg_config_home_path.join(path))
                } else {
                    get_xdg_data_home_value()
                        .map(|xdg_data_home_path| xdg_data_home_path.join(path))
                }
            }
            _ => None, // Unsupported prefix
        },
        None => Some(path),
    }
}

#[cfg(all(feature = "std", feature = "parsing"))]
fn FcScanDirectories() -> Option<Vec<(FcPattern, FcFontPath)>> {
    use std::fs;
    use std::path::Path;

    const BASE_FONTCONFIG_PATH: &str = "/etc/fonts/fonts.conf";

    if !Path::new(BASE_FONTCONFIG_PATH).exists() {
        return None;
    }

    let mut font_paths = Vec::with_capacity(32);
    let mut paths_to_visit = vec![(None, PathBuf::from(BASE_FONTCONFIG_PATH))];

    while let Some((prefix, mut path_to_visit)) = paths_to_visit.pop() {
        path_to_visit = match process_path(&prefix, path_to_visit, true) {
            Some(path) => path,
            None => continue,
        };

        let metadata = match fs::metadata(path_to_visit.as_path()) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        if metadata.is_file() {
            let xml_utf8 = match fs::read_to_string(path_to_visit.as_path()) {
                Ok(xml_utf8) => xml_utf8,
                Err(_) => continue,
            };

            ParseFontsConf(xml_utf8.as_str(), &mut paths_to_visit, &mut font_paths);
        } else if metadata.is_dir() {
            let dir_entries = match fs::read_dir(path_to_visit) {
                Ok(dir_entries) => dir_entries,
                Err(_) => continue,
            };

            for dir_entry in dir_entries {
                if let Ok(dir_entry) = dir_entry {
                    let entry_path = dir_entry.path();

                    // `fs::metadata` traverses symbolic links
                    let metadata = match fs::metadata(entry_path.as_path()) {
                        Ok(metadata) => metadata,
                        Err(_) => continue,
                    };

                    if metadata.is_file() {
                        if let Some(file_name) = entry_path.file_name() {
                            let file_name_str = file_name.to_string_lossy();
                            if file_name_str.starts_with(|c: char| c.is_ascii_digit())
                                && file_name_str.ends_with(".conf")
                            {
                                paths_to_visit.push((None, entry_path));
                            }
                        }
                    }
                } else {
                    return None;
                }
            }
        }
    }

    if font_paths.is_empty() {
        return None;
    }

    Some(FcScanDirectoriesInner(font_paths.as_slice()))
}

// Parses the fonts.conf file
#[cfg(all(feature = "std", feature = "parsing"))]
fn ParseFontsConf(
    input: &str,
    paths_to_visit: &mut Vec<(Option<String>, PathBuf)>,
    font_paths: &mut Vec<(Option<String>, String)>,
) -> Option<()> {
    use xmlparser::Token::*;
    use xmlparser::Tokenizer;

    const TAG_INCLUDE: &str = "include";
    const TAG_DIR: &str = "dir";
    const ATTRIBUTE_PREFIX: &str = "prefix";

    let mut current_prefix: Option<&str> = None;
    let mut current_path: Option<&str> = None;
    let mut is_in_include = false;
    let mut is_in_dir = false;

    for token in Tokenizer::from(input) {
        let token = token.ok()?;
        match token {
            ElementStart { local, .. } => {
                if is_in_include || is_in_dir {
                    return None; /* error: nested tags */
                }

                match local.as_str() {
                    TAG_INCLUDE => {
                        is_in_include = true;
                    }
                    TAG_DIR => {
                        is_in_dir = true;
                    }
                    _ => continue,
                }

                current_path = None;
            }
            Text { text, .. } => {
                let text = text.as_str().trim();
                if text.is_empty() {
                    continue;
                }
                if is_in_include || is_in_dir {
                    current_path = Some(text);
                }
            }
            Attribute { local, value, .. } => {
                if !is_in_include && !is_in_dir {
                    continue;
                }
                // attribute on <include> or <dir> node
                if local.as_str() == ATTRIBUTE_PREFIX {
                    current_prefix = Some(value.as_str());
                }
            }
            ElementEnd { end, .. } => {
                let end_tag = match end {
                    xmlparser::ElementEnd::Close(_, a) => a,
                    _ => continue,
                };

                match end_tag.as_str() {
                    TAG_INCLUDE => {
                        if !is_in_include {
                            continue;
                        }

                        if let Some(current_path) = current_path.as_ref() {
                            paths_to_visit.push((
                                current_prefix.map(ToOwned::to_owned),
                                PathBuf::from(*current_path),
                            ));
                        }
                    }
                    TAG_DIR => {
                        if !is_in_dir {
                            continue;
                        }

                        if let Some(current_path) = current_path.as_ref() {
                            font_paths.push((
                                current_prefix.map(ToOwned::to_owned),
                                (*current_path).to_owned(),
                            ));
                        }
                    }
                    _ => continue,
                }

                is_in_include = false;
                is_in_dir = false;
                current_path = None;
                current_prefix = None;
            }
            _ => {}
        }
    }

    Some(())
}

#[cfg(all(feature = "std", feature = "parsing"))]
fn FcScanDirectoriesInner(paths: &[(Option<String>, String)]) -> Vec<(FcPattern, FcFontPath)> {
    #[cfg(feature = "multithreading")]
    {
        use rayon::prelude::*;

        // scan directories in parallel
        paths
            .par_iter()
            .filter_map(|(prefix, p)| {
                if let Some(path) = process_path(prefix, PathBuf::from(p), false) {
                    Some(FcScanSingleDirectoryRecursive(path))
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }
    #[cfg(not(feature = "multithreading"))]
    {
        paths
            .iter()
            .filter_map(|(prefix, p)| {
                if let Some(path) = process_path(prefix, PathBuf::from(p), false) {
                    Some(FcScanSingleDirectoryRecursive(path))
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }
}

#[cfg(all(feature = "std", feature = "parsing"))]
fn FcScanSingleDirectoryRecursive(dir: PathBuf) -> Vec<(FcPattern, FcFontPath)> {
    let mut files_to_parse = Vec::new();
    let mut dirs_to_parse = vec![dir];

    'outer: loop {
        let mut new_dirs_to_parse = Vec::new();

        'inner: for dir in dirs_to_parse.clone() {
            let dir = match std::fs::read_dir(dir) {
                Ok(o) => o,
                Err(_) => continue 'inner,
            };

            for (path, pathbuf) in dir.filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                let pathbuf = path.to_path_buf();
                Some((path, pathbuf))
            }) {
                if path.is_dir() {
                    new_dirs_to_parse.push(pathbuf);
                } else {
                    files_to_parse.push(pathbuf);
                }
            }
        }

        if new_dirs_to_parse.is_empty() {
            break 'outer;
        } else {
            dirs_to_parse = new_dirs_to_parse;
        }
    }

    FcParseFontFiles(&files_to_parse)
}

#[cfg(all(feature = "std", feature = "parsing"))]
fn FcParseFontFiles(files_to_parse: &[PathBuf]) -> Vec<(FcPattern, FcFontPath)> {
    let result = {
        #[cfg(feature = "multithreading")]
        {
            use rayon::prelude::*;

            files_to_parse
                .par_iter()
                .filter_map(|file| FcParseFont(file))
                .collect::<Vec<Vec<_>>>()
        }
        #[cfg(not(feature = "multithreading"))]
        {
            files_to_parse
                .iter()
                .filter_map(|file| FcParseFont(file))
                .collect::<Vec<Vec<_>>>()
        }
    };

    result.into_iter().flat_map(|f| f.into_iter()).collect()
}

#[cfg(all(feature = "std", feature = "parsing"))]
fn FcParseFont(filepath: &PathBuf) -> Option<Vec<(FcPattern, FcFontPath)>> {
    use allsorts::{
        binary::read::ReadScope,
        font_data::FontData,
        get_name::fontcode_get_name,
        post::PostTable,
        tables::{
            os2::Os2, FontTableProvider, HeadTable, HheaTable, HmtxTable, MaxpTable, NameTable,
        },
        tag,
    };
    #[cfg(all(not(target_family = "wasm"), feature = "std"))]
    use mmapio::MmapOptions;
    use std::collections::BTreeSet;
    use std::fs::File;

    const FONT_SPECIFIER_NAME_ID: u16 = 4;
    const FONT_SPECIFIER_FAMILY_ID: u16 = 1;

    // font_index = 0 - TODO: iterate through fonts in font file properly!
    let font_index = 0;

    // try parsing the font file and see if the postscript name matches
    let file = File::open(filepath).ok()?;
    #[cfg(all(not(target_family = "wasm"), feature = "std"))]
    let font_bytes = unsafe { MmapOptions::new().map(&file).ok()? };
    #[cfg(not(all(not(target_family = "wasm"), feature = "std")))]
    let font_bytes = std::fs::read(filepath).ok()?;
    let scope = ReadScope::new(&font_bytes[..]);
    let font_file = scope.read::<FontData<'_>>().ok()?;
    let provider = font_file.table_provider(font_index).ok()?;

    let head_data = provider.table_data(tag::HEAD).ok()??.into_owned();
    let head_table = ReadScope::new(&head_data).read::<HeadTable>().ok()?;

    let is_bold = head_table.is_bold();
    let is_italic = head_table.is_italic();
    let mut detected_monospace = None;

    let post_data = provider.table_data(tag::POST).ok()??;
    if let Ok(post_table) = ReadScope::new(&post_data).read::<PostTable>() {
        // isFixedPitch here - https://learn.microsoft.com/en-us/typography/opentype/spec/post#header
        detected_monospace = Some(post_table.header.is_fixed_pitch != 0);
    }

    if detected_monospace.is_none() {
        // https://learn.microsoft.com/en-us/typography/opentype/spec/os2#panose
        // Table 20 here - https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6OS2.html
        let os2_data = provider.table_data(tag::OS_2).ok()??;
        let os2_table = ReadScope::new(&os2_data)
            .read_dep::<Os2>(os2_data.len())
            .ok()?;
        let monospace = os2_table.panose[0] == 2;
        detected_monospace = Some(monospace);
    }

    if detected_monospace.is_none() {
        let hhea_data = provider.table_data(tag::HHEA).ok()??;
        let hhea_table = ReadScope::new(&hhea_data).read::<HheaTable>().ok()?;
        let maxp_data = provider.table_data(tag::MAXP).ok()??;
        let maxp_table = ReadScope::new(&maxp_data).read::<MaxpTable>().ok()?;
        let hmtx_data = provider.table_data(tag::HMTX).ok()??;
        let hmtx_table = ReadScope::new(&hmtx_data)
            .read_dep::<HmtxTable<'_>>((
                usize::from(maxp_table.num_glyphs),
                usize::from(hhea_table.num_h_metrics),
            ))
            .ok()?;

        let mut monospace = true;
        let mut last_advance = 0;
        for i in 0..hhea_table.num_h_metrics as usize {
            let advance = hmtx_table.h_metrics.read_item(i).ok()?.advance_width;
            if i > 0 && advance != last_advance {
                monospace = false;
                break;
            }
            last_advance = advance;
        }

        detected_monospace = Some(monospace);
    }

    let is_monospace = detected_monospace.unwrap_or(false);

    let name_data = provider.table_data(tag::NAME).ok()??.into_owned();
    let name_table = ReadScope::new(&name_data).read::<NameTable>().ok()?;

    // one font can support multiple patterns
    let mut f_family = None;

    let patterns = name_table
        .name_records
        .iter() // TODO: par_iter
        .filter_map(|name_record| {
            let name_id = name_record.name_id;
            if name_id == FONT_SPECIFIER_FAMILY_ID {
                let family = fontcode_get_name(&name_data, FONT_SPECIFIER_FAMILY_ID).ok()??;
                f_family = Some(family);
                None
            } else if name_id == FONT_SPECIFIER_NAME_ID {
                let family = f_family.as_ref()?;
                let name = fontcode_get_name(&name_data, FONT_SPECIFIER_NAME_ID).ok()??;
                if name.to_bytes().is_empty() {
                    None
                } else {
                    Some((
                        FcPattern {
                            name: Some(String::from_utf8_lossy(name.to_bytes()).to_string()),
                            family: Some(String::from_utf8_lossy(family.as_bytes()).to_string()),
                            bold: if is_bold {
                                PatternMatch::True
                            } else {
                                PatternMatch::False
                            },
                            italic: if is_italic {
                                PatternMatch::True
                            } else {
                                PatternMatch::False
                            },
                            monospace: if is_monospace {
                                PatternMatch::True
                            } else {
                                PatternMatch::False
                            },
                            ..Default::default() // TODO!
                        },
                        font_index,
                    ))
                }
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();

    Some(
        patterns
            .into_iter()
            .map(|(pat, index)| {
                (
                    pat,
                    FcFontPath {
                        path: filepath.to_string_lossy().to_string(),
                        font_index: index,
                    },
                )
            })
            .collect(),
    )
}

#[cfg(all(feature = "std", feature = "parsing"))]
pub fn get_font_name(font_path: &FcFontPath) -> Option<(String, String)> {
    use allsorts::{
        binary::read::ReadScope,
        font_data::FontData,
        get_name::fontcode_get_name,
        tables::{FontTableProvider, NameTable},
        tag,
    };

    const FONT_SPECIFIER_NAME_ID: u16 = 4;
    const FONT_SPECIFIER_FAMILY_ID: u16 = 1;

    let font_bytes = std::fs::read(&font_path.path).ok()?;
    let scope = ReadScope::new(&font_bytes[..]);
    let font_file = scope.read::<FontData<'_>>().ok()?;
    let provider = font_file.table_provider(font_path.font_index).ok()?;

    let name_data = provider.table_data(tag::NAME).ok()??.into_owned();
    let name_table = ReadScope::new(&name_data).read::<NameTable>().ok()?;

    let mut font_family = None;
    let mut font_name = None;

    for name_record in name_table.name_records.iter() {
        match name_record.name_id {
            FONT_SPECIFIER_FAMILY_ID => {
                if let Ok(Some(family)) = fontcode_get_name(&name_data, FONT_SPECIFIER_FAMILY_ID) {
                    font_family = Some(String::from_utf8_lossy(family.as_bytes()).to_string());
                }
            }
            FONT_SPECIFIER_NAME_ID => {
                if let Ok(Some(name)) = fontcode_get_name(&name_data, FONT_SPECIFIER_NAME_ID) {
                    font_name = Some(String::from_utf8_lossy(name.to_bytes()).to_string());
                }
            }
            _ => continue,
        }

        if font_family.is_some() && font_name.is_some() {
            break;
        }
    }

    if let (Some(family), Some(name)) = (font_family, font_name) {
        Some((family, name))
    } else {
        None
    }
}
