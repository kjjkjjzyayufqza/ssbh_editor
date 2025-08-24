# Export NUMDLB Scene Feature Design

## Overview

This document outlines the implementation plan for adding "Export NUMDLB Scene" functionality to SSBH Editor. This feature allows users to export a complete scene containing the three core SSBH files that make up a model:

- **model.numshb** - Mesh vertex data
- **model.numdlb** - Model data (material assignments and mesh references)  
- **model.nusktb** - Skeleton data

Unlike the existing GLTF export, this feature exports native SSBH format files that can be used directly in Smash Ultimate modding workflows.

## File Structure Changes

### 1. Create New Export Module

**File**: `src/export/numdlb_scene.rs`

```rust
use anyhow::Result;
use ssbh_data::{mesh_data::MeshData, modl_data::ModlData, skel_data::SkelData};
use ssbh_wgpu::ModelFolder;
use std::path::Path;

/// Configuration for NUMDLB scene export
#[derive(Debug, Clone)]
pub struct SceneExportConfig {
    pub base_filename: String,
    pub export_mesh: bool,
    pub export_skeleton: bool,
    pub export_modl: bool,
    pub output_directory: std::path::PathBuf,
}

impl Default for SceneExportConfig {
    fn default() -> Self {
        Self {
            base_filename: "model".to_string(),
            export_mesh: true,
            export_skeleton: true,
            export_modl: true,
            output_directory: std::path::PathBuf::new(),
        }
    }
}

/// Export scene configuration dialog state
#[derive(Debug, Default)]
pub struct SceneExportDialogState {
    pub config: SceneExportConfig,
    pub is_open: bool,
}

/// Export a model folder's scene data to NUMDLB format files
pub fn export_scene_to_numdlb(
    model_folder: &ModelFolder,
    config: &SceneExportConfig,
) -> Result<Vec<String>> {
    let mut exported_files = Vec::new();
    
    // Export mesh data (.numshb)
    if config.export_mesh {
        if let Some((_, Some(mesh_data))) = model_folder.meshes.first() {
            let mesh_path = config.output_directory.join(format!("{}.numshb", config.base_filename));
            mesh_data.write_to_file(&mesh_path)?;
            exported_files.push(format!("{}.numshb", config.base_filename));
        }
    }
    
    // Export skeleton data (.nusktb)
    if config.export_skeleton {
        if let Some((_, Some(skel_data))) = model_folder.skels.first() {
            let skel_path = config.output_directory.join(format!("{}.nusktb", config.base_filename));
            skel_data.write_to_file(&skel_path)?;
            exported_files.push(format!("{}.nusktb", config.base_filename));
        }
    }
    
    // Export model data (.numdlb)
    if config.export_modl {
        if let Some((_, Some(modl_data))) = model_folder.modls.first() {
            let modl_path = config.output_directory.join(format!("{}.numdlb", config.base_filename));
            modl_data.write_to_file(&modl_path)?;
            exported_files.push(format!("{}.numdlb", config.base_filename));
        }
    }
    
    Ok(exported_files)
}

/// Show the scene export configuration dialog
pub fn show_scene_export_dialog(
    ctx: &egui::Context,
    state: &mut SceneExportDialogState,
) -> Option<SceneExportConfig> {
    let mut result = None;
    
    if state.is_open {
        egui::Window::new("Export NUMDLB Scene")
            .open(&mut state.is_open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                egui::Grid::new("export_config_grid")
                    .num_columns(2)
                    .spacing([10.0, 10.0])
                    .show(ui, |ui| {
                        ui.label("Base filename:");
                        ui.text_edit_singleline(&mut state.config.base_filename);
                        ui.end_row();
                        
                        ui.label("Export mesh (.numshb):");
                        ui.checkbox(&mut state.config.export_mesh, "");
                        ui.end_row();
                        
                        ui.label("Export skeleton (.nusktb):");
                        ui.checkbox(&mut state.config.export_skeleton, "");
                        ui.end_row();
                        
                        ui.label("Export model (.numdlb):");
                        ui.checkbox(&mut state.config.export_modl, "");
                        ui.end_row();
                    });
                
                ui.separator();
                
                ui.horizontal(|ui| {
                    if ui.button("Select Output Directory").clicked() {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            state.config.output_directory = dir;
                        }
                    }
                    
                    if !state.config.output_directory.as_os_str().is_empty() {
                        ui.label(format!("Output: {}", state.config.output_directory.display()));
                    } else {
                        ui.label("No output directory selected");
                    }
                });
                
                ui.separator();
                
                ui.horizontal(|ui| {
                    let can_export = !state.config.output_directory.as_os_str().is_empty() 
                        && !state.config.base_filename.is_empty()
                        && (state.config.export_mesh || state.config.export_skeleton || state.config.export_modl);
                    
                    if ui.add_enabled(can_export, egui::Button::new("Export")).clicked() {
                        result = Some(state.config.clone());
                        state.is_open = false;
                    }
                    
                    if ui.button("Cancel").clicked() {
                        state.is_open = false;
                    }
                });
            });
    }
    
    result
}
```

### 2. Update Export Module

**File**: `src/export/mod.rs`

```rust
pub mod gltf;
pub mod numdlb_scene;
```

### 3. Update Application State

**File**: `src/app.rs`

Add to the `SsbhApp` struct:

```rust
// Add to imports
use crate::export::numdlb_scene::{SceneExportDialogState, SceneExportConfig, export_scene_to_numdlb, show_scene_export_dialog};

// Add to SsbhApp struct
pub struct SsbhApp {
    // ... existing fields ...
    scene_export_dialog: SceneExportDialogState,
    pending_scene_export: Option<(usize, SceneExportConfig)>, // (model_index, config)
}

// Add to SsbhApp::new() method
scene_export_dialog: SceneExportDialogState::default(),
pending_scene_export: None,
```

### 4. Update Menu System

**File**: `src/app/menu.rs`

Update the export menu section:

```rust
ui.menu_button("Export", |ui| {
    if button(ui, "Export Scene to GLTF...").clicked() {
        if let Some(file) = FileDialog::new()
            .add_filter("GLTF", &["gltf"])
            .save_file()
        {
            app.export_gltf_path = Some(file);
        }
    }
    
    ui.separator();
    
    if button(ui, "Export NUMDLB Scene...").clicked() {
        app.scene_export_dialog.is_open = true;
    }
});
```

### 5. Update Main Application Loop

**File**: `src/app.rs`

Add to the `render` method, after the GLTF export handling:

```rust
// Handle NUMDLB scene export
if let Some((model_index, config)) = &self.pending_scene_export {
    if let Some(model) = self.models.get(*model_index) {
        match export_scene_to_numdlb(&model.model, config) {
            Ok(exported_files) => {
                log::info!(
                    "Successfully exported NUMDLB scene to {}: {:?}", 
                    config.output_directory.display(),
                    exported_files
                );
            }
            Err(e) => {
                error!("Error exporting NUMDLB scene: {e}");
            }
        }
    } else {
        error!("No model selected for export");
    }
    self.pending_scene_export = None;
}
```

Add to the `show_windows` method:

```rust
// Show scene export dialog
if let Some(config) = show_scene_export_dialog(ctx, &mut self.scene_export_dialog) {
    if let Some(selected_index) = self.ui_state.selected_folder_index {
        self.pending_scene_export = Some((selected_index, config));
    } else if !self.models.is_empty() {
        // If no model is selected, export the first one
        self.pending_scene_export = Some((0, config));
    }
}
```

### 6. Add Dependencies

**File**: `Cargo.toml`

## Implementation Steps

1. **Create the export module** (`src/export/numdlb_scene.rs`)
   - Implement `SceneExportConfig` struct
   - Implement `SceneExportDialogState` struct  
   - Implement `export_scene_to_numdlb` function
   - Implement `show_scene_export_dialog` function

2. **Update export module declaration** (`src/export/mod.rs`)
   - Add `pub mod numdlb_scene;`

3. **Update application state** (`src/app.rs`)
   - Add dialog state and pending export to `SsbhApp`
   - Add imports for new export functionality

4. **Update menu system** (`src/app/menu.rs`)
   - Add "Export NUMDLB Scene..." menu item
   - Set dialog open state when clicked

5. **Update main loop** (`src/app.rs`)
   - Add dialog handling in `show_windows`
   - Add export processing in `render`

6. **Test the implementation**
   - Verify dialog appears when menu item is clicked
   - Test export functionality with various configurations
   - Ensure exported files can be loaded back into SSBH Editor

## User Experience

### Export Dialog Features

1. **Base Filename**: Text input for the base name (default: "model")
   - Results in files like `model.numshb`, `model.nusktb`, `model.numdlb`

2. **File Type Selection**: Checkboxes for each file type
   - Export mesh (.numshb) - checked by default
   - Export skeleton (.nusktb) - checked by default  
   - Export model (.numdlb) - checked by default

3. **Output Directory**: Button to select destination folder
   - Shows selected path
   - Required before export can proceed

4. **Export/Cancel Buttons**:
   - Export button disabled until valid configuration
   - Cancel button closes dialog without action

### Export Process

1. User clicks "Export NUMDLB Scene..." from File > Export menu
2. Configuration dialog opens
3. User configures export settings
4. User selects output directory
5. User clicks Export
6. Files are written to selected directory
7. Success/error message logged

### Error Handling

- Invalid output directory
- File write permissions
- Missing source data (mesh, skeleton, model)
- Filename validation

## Future Enhancements

1. **Batch Export**: Export multiple models at once
2. **Custom Extensions**: Allow custom file extensions
3. **Export Validation**: Verify exported files can be loaded
4. **Progress Indication**: Show progress for large exports
5. **Recent Locations**: Remember recent export directories
6. **Export Presets**: Save/load export configurations

## Testing Considerations

1. Test with models that have all three file types
2. Test with models missing some file types
3. Test with various filename inputs (special characters, long names)
4. Test export to different directory structures
5. Test canceling the export dialog
6. Test selecting different models before export
7. Verify exported files are valid SSBH format
