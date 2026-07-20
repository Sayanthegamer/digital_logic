use crate::engine::ChipBlueprint;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;

/// A folder within the global chip library for organisation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ChipFolder {
    pub name: String,
    /// Optional accent colour for the folder (RGBA)
    #[serde(default)]
    pub color: Option<[f32; 4]>,
    pub chips: Vec<ChipBlueprint>,
}

/// The global chip library, persisted across projects.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GlobalLibrary {
    /// Organised chip folders
    #[serde(default)]
    pub folders: Vec<ChipFolder>,
    /// Chips not yet placed in any folder
    #[serde(default)]
    pub ungrouped: Vec<ChipBlueprint>,
}

impl GlobalLibrary {
    /// Compute a content hash for a ChipBlueprint based on its components and connections.
    /// Uses index_map to normalize SubChip indices for accurate deduplication.
    pub fn blueprint_hash(bp: &ChipBlueprint, index_map: &HashMap<usize, usize>) -> u64 {
        let mut hasher = DefaultHasher::new();
        bp.inputs.hash(&mut hasher);
        bp.outputs.hash(&mut hasher);
        for comp in &bp.components {
            match &comp.component_type {
                crate::engine::ComponentType::SubChip(idx) => {
                    let mapped_idx = index_map.get(idx).cloned().unwrap_or(*idx);
                    format!("SubChip({})", mapped_idx).hash(&mut hasher);
                }
                other => {
                    format!("{:?}", other).hash(&mut hasher);
                }
            }
        }
        for conn in &bp.connections {
            format!("{:?}{:?}", conn.source, conn.target).hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Flatten the library into a single Vec for the engine, preserving order:
    /// ungrouped chips first, then folder chips in folder order.
    pub fn to_flat_list(&self) -> Vec<ChipBlueprint> {
        let mut result = self.ungrouped.clone();
        for folder in &self.folders {
            result.extend(folder.chips.clone());
        }
        result
    }

    /// Import chips from a project's library, mapping and remapping their sub-chip dependencies.
    /// Returns the index mapping (old index -> new flat index).
    pub fn import_from_project(&mut self, project_chips: &[ChipBlueprint]) -> HashMap<usize, usize> {
        let mut index_map = HashMap::new();

        let existing_flat = self.to_flat_list();
        let mut existing_hashes: Vec<u64> = existing_flat
            .iter()
            .map(|bp| Self::blueprint_hash(bp, &HashMap::new()))
            .collect();

        let mut existing_names: Vec<String> = existing_flat
            .iter()
            .map(|bp| bp.name.clone())
            .collect();

        let mut new_imports = Vec::new();
        let new_flat_len = existing_flat.len();

        // 1. Map all old indices to their new flat indices
        for (old_idx, chip) in project_chips.iter().enumerate() {
            let hash = Self::blueprint_hash(chip, &index_map);

            if let Some(pos) = existing_hashes.iter().position(|&h| h == hash) {
                // Same content already exists in the global library, map to its flat index
                index_map.insert(old_idx, pos);
            } else {
                // Different content — needs to be imported.
                let new_idx = new_flat_len + new_imports.len();
                index_map.insert(old_idx, new_idx);

                let mut import_chip = chip.clone();
                if existing_names.contains(&import_chip.name) {
                    // Auto-rename to avoid confusion
                    let mut suffix = 1;
                    loop {
                        let candidate = format!("{}_{}", chip.name, suffix);
                        if !existing_names.contains(&candidate) {
                            import_chip.name = candidate;
                            break;
                        }
                        suffix += 1;
                    }
                }

                existing_names.push(import_chip.name.clone());
                existing_hashes.push(hash);
                new_imports.push(import_chip);
            }
        }

        // 2. Remap all internal components of the newly imported chips
        for chip in &mut new_imports {
            for comp in &mut chip.components {
                if let crate::engine::ComponentType::SubChip(ref mut sub_idx) = comp.component_type {
                    if let Some(&new_sub_idx) = index_map.get(sub_idx) {
                        *sub_idx = new_sub_idx;
                    }
                }
            }
        }

        // 3. Append the remapped new chips to the ungrouped section
        self.ungrouped.extend(new_imports);

        index_map
    }

    /// Find the (folder_index, chip_index) for a chip at a given flat index.
    /// Returns None if out of bounds.
    pub fn flat_index_to_location(&self, flat_idx: usize) -> Option<ChipLocation> {
        if flat_idx < self.ungrouped.len() {
            return Some(ChipLocation::Ungrouped(flat_idx));
        }
        let mut remaining = flat_idx - self.ungrouped.len();
        for (fi, folder) in self.folders.iter().enumerate() {
            if remaining < folder.chips.len() {
                return Some(ChipLocation::InFolder(fi, remaining));
            }
            remaining -= folder.chips.len();
        }
        None
    }

    /// Move a chip from one location to another folder (or ungrouped).
    pub fn move_chip(&mut self, from: ChipLocation, to_folder: Option<usize>) {
        let chip = match from {
            ChipLocation::Ungrouped(idx) => {
                if idx < self.ungrouped.len() {
                    self.ungrouped.remove(idx)
                } else {
                    return;
                }
            }
            ChipLocation::InFolder(fi, ci) => {
                if fi < self.folders.len() && ci < self.folders[fi].chips.len() {
                    self.folders[fi].chips.remove(ci)
                } else {
                    return;
                }
            }
        };

        match to_folder {
            Some(fi) if fi < self.folders.len() => {
                self.folders[fi].chips.push(chip);
            }
            _ => {
                self.ungrouped.push(chip);
            }
        }
    }

    /// Remove a chip at the given flat index. Returns the removed chip if any.
    pub fn remove_at_flat_index(&mut self, flat_idx: usize) -> Option<ChipBlueprint> {
        match self.flat_index_to_location(flat_idx) {
            Some(ChipLocation::Ungrouped(idx)) => Some(self.ungrouped.remove(idx)),
            Some(ChipLocation::InFolder(fi, ci)) => Some(self.folders[fi].chips.remove(ci)),
            None => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ChipLocation {
    Ungrouped(usize),
    InFolder(usize, usize), // (folder_idx, chip_idx_within_folder)
}

// --- Persistence ---

/// Get the global library directory path.
#[cfg(not(target_os = "android"))]
pub fn get_global_library_dir() -> std::path::PathBuf {
    // Use APPDATA on Windows, XDG_DATA_HOME or ~/.local/share on Linux, ~/Library/Application Support on macOS
    if let Ok(appdata) = std::env::var("APPDATA") {
        std::path::PathBuf::from(appdata).join("logic_simulator")
    } else if let Ok(home) = std::env::var("HOME") {
        std::path::PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("logic_simulator")
    } else {
        // Fallback to current directory
        std::path::PathBuf::from(".")
    }
}

/// Get the global library file path.
#[cfg(not(target_os = "android"))]
pub fn get_global_library_path() -> std::path::PathBuf {
    get_global_library_dir().join("global_library.json")
}

/// Load the global library from disk. Returns default (empty) if not found.
pub fn load_global_library() -> GlobalLibrary {
    #[cfg(not(target_os = "android"))]
    {
        let path = get_global_library_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(lib) = serde_json::from_str::<GlobalLibrary>(&content) {
                    return lib;
                } else {
                    eprintln!("Failed to parse global library at {:?}", path);
                }
            }
        }
    }
    GlobalLibrary::default()
}

/// Save the global library to disk.
pub fn save_global_library(lib: &GlobalLibrary) {
    #[cfg(target_os = "android")]
    let _ = lib;
    #[cfg(not(target_os = "android"))]
    {
        let dir = get_global_library_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!("Failed to create global library dir {:?}: {}", dir, e);
            return;
        }
        let path = get_global_library_path();
        match serde_json::to_string_pretty(lib) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    eprintln!("Failed to write global library to {:?}: {}", path, e);
                }
            }
            Err(e) => {
                eprintln!("Failed to serialize global library: {}", e);
            }
        }
    }
}
