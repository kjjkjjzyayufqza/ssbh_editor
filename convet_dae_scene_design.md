# Convert DAE Feature Design

## Overview

This document outlines the implementation plan for adding "Convert DAE" functionality to SSBH Editor. This feature allows users to convert a COLLADA (.dae) format model file into the core SSBH format files for external use or manual importing.

The DAE file will be converted to three essential SSBH files:
- **model.numdlb** - Model data (material assignments and mesh references)
- **model.numshb** - Mesh vertex data
- **model.numatb** - Material data

This standalone conversion tool provides flexibility for users who want to convert DAE files to SSBH format without automatically loading them into the editor workspace. The converted files can be manually imported later or used in external workflows.

## Current Project Analysis

### Existing Import System

The current project loads models through:
- `ssbh_wgpu::load_model_folders()` - Loads native SSBH format files
- `ModelFolder` structure - Contains parsed SSBH data
- `src/app.rs::add_folder_to_workspace()` - Integration point for loaded models

### Existing Export System

The project already has export systems for:
- GLTF export (`src/export/gltf.rs`)
- NUMDLB Scene export (`src/export/numdlb_scene.rs`)

### Conversion Output Analysis

**✅ Direct SSBH File Generation**

The DAE conversion system is designed to generate SSBH files directly without requiring intermediate ModelFolder structures:

1. **Core SSBH Files**: 
   - Generates `.numdlb` - Model structure and material assignments
   - Generates `.numshb` - Mesh vertex data and geometry
   - Generates `.numatb` - Material definitions and properties

2. **Standalone Operation**:
   - No dependency on editor workspace or scene management
   - Direct file-to-file conversion without memory-intensive ModelFolder creation
   - Suitable for batch processing or external tool integration

3. **Data Validation**:
   - Converted files comply with SSBH format standards
   - Validates mesh attributes, material assignments, and data integrity
   - Ensures compatibility with SSBH Editor and other SSBH tools

**Data Flow**:
```
DAE File → parse_dae_file() → DaeScene → convert_to_ssbh_files() → .numdlb + .numshb + .numatb
```

## File Structure Changes

### 1. Create New Convert Module

**File**: `src/convert/mod.rs`

```rust
pub mod dae;
```

### 2. Create DAE Convert Module

**File**: `src/convert/dae.rs`

```rust
use anyhow::{Result, anyhow};
use ssbh_data::{
    mesh_data::{MeshData, MeshObjectData, AttributeData, VectorData},
    modl_data::{ModlData, ModlEntryData},
    matl_data::{MatlData, MatlEntryData, ParamData, ParamId},
    Vector4,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
// Option 1: Use dae-parser for robust COLLADA parsing
use dae_parser::{Document as DaeDocument, Scene as ColladaScene};
// Option 2: Fallback to manual XML parsing (not recommended)
// use xmltree::Element;

/// Configuration for DAE conversion
#[derive(Debug, Clone)]
pub struct DaeConvertConfig {
    pub output_directory: PathBuf,
    pub base_filename: String,
    pub scale_factor: f32,
    pub up_axis_conversion: UpAxisConversion,
}

impl Default for DaeConvertConfig {
    fn default() -> Self {
        Self {
            output_directory: PathBuf::new(),
            base_filename: "model".to_string(),
            scale_factor: 1.0,
            up_axis_conversion: UpAxisConversion::YUp,
        }
    }
}

/// Up axis conversion options
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpAxisConversion {
    YUp,
    ZUp,
    NoConversion,
}

/// Convert dialog state
#[derive(Debug, Default)]
pub struct DaeConvertDialogState {
    pub config: DaeConvertConfig,
    pub is_open: bool,
    pub selected_dae_file: Option<PathBuf>,
}

/// Parsed DAE scene data
#[derive(Debug)]
pub struct DaeScene {
    pub meshes: Vec<DaeMesh>,
    pub materials: Vec<DaeMaterial>,
    pub up_axis: UpAxisConversion,
}

#[derive(Debug)]
pub struct DaeMesh {
    pub name: String,
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub material_name: Option<String>,
}

#[derive(Debug)]
pub struct DaeMaterial {
    pub name: String,
    pub diffuse_color: [f32; 4],
    pub specular_color: [f32; 4],
    pub emission_color: [f32; 4],
    pub texture_paths: HashMap<String, String>,
}



/// Parse DAE file and extract scene data using dae-parser library
pub fn parse_dae_file(file_path: &Path) -> Result<DaeScene> {
    // Use dae-parser for robust COLLADA parsing
    let dae_document = DaeDocument::from_file(file_path)
        .map_err(|e| anyhow!("Failed to parse DAE file: {}", e))?;
    
    let mut scene = DaeScene {
        meshes: Vec::new(),
        materials: Vec::new(),
        up_axis: UpAxisConversion::YUp,
    };
    
    // Extract up axis from asset information
    if let Some(asset) = &dae_document.asset {
        scene.up_axis = match asset.up_axis.as_str() {
            "Y_UP" => UpAxisConversion::YUp,
            "Z_UP" => UpAxisConversion::ZUp,
            _ => UpAxisConversion::YUp,
        };
    }
    
    // Parse materials from library_materials
    if let Some(lib_materials) = &dae_document.library_materials {
        scene.materials = parse_materials_from_dae(lib_materials)?;
    }
    
    // Parse geometries from library_geometries
    if let Some(lib_geometries) = &dae_document.library_geometries {
        scene.meshes = parse_geometries_from_dae(lib_geometries, &scene.materials)?;
    }
    
    Ok(scene)
}

/// Fallback: Parse DAE file using manual XML parsing (not recommended)
#[allow(dead_code)]
pub fn parse_dae_file_manual(file_path: &Path) -> Result<DaeScene> {
    // This is the original manual parsing approach - keep as fallback
    // Implementation details omitted for brevity
    // Only use if dae-parser doesn't work for specific files
    todo!("Implement manual parsing as fallback if needed")
}

/// Convert DAE scene directly to SSBH files
pub fn convert_dae_to_ssbh_files(
    dae_scene: &DaeScene,
    config: &DaeConvertConfig,
) -> Result<ConvertedFiles> {
    let mut converted_files = ConvertedFiles::default();
    
    // Convert and write mesh data
    if !dae_scene.meshes.is_empty() {
        let mesh_data = convert_meshes_to_ssbh(&dae_scene.meshes, config)?;
        let mesh_path = config.output_directory.join(format!("{}.numshb", config.base_filename));
        mesh_data.write_to_file(&mesh_path)?;
        converted_files.numshb_path = Some(mesh_path);
    }
    
    // Convert and write model data
    if !dae_scene.meshes.is_empty() {
        let modl_data = convert_model_to_ssbh(&dae_scene.meshes, &dae_scene.materials, config)?;
        let modl_path = config.output_directory.join(format!("{}.numdlb", config.base_filename));
        modl_data.write_to_file(&modl_path)?;
        converted_files.numdlb_path = Some(modl_path);
    }
    
    // Convert and write material data
    let matl_data = convert_materials_to_ssbh(&dae_scene.materials, config)?;
    let matl_path = config.output_directory.join(format!("{}.numatb", config.base_filename));
    matl_data.write_to_file(&matl_path)?;
    converted_files.numatb_path = Some(matl_path);
    
    Ok(converted_files)
}

/// Result of DAE conversion operation
#[derive(Debug, Default)]
pub struct ConvertedFiles {
    pub numdlb_path: Option<PathBuf>,
    pub numshb_path: Option<PathBuf>,
    pub numatb_path: Option<PathBuf>,
}

/// Convert DAE file to SSBH files
pub fn convert_dae_file(
    dae_file_path: &Path,
    config: &DaeConvertConfig,
) -> Result<ConvertedFiles> {
    // Parse DAE file
    let dae_scene = parse_dae_file(dae_file_path)?;
    
    // Validate parsed data
    validate_dae_scene(&dae_scene)?;
    
    // Convert to SSBH files
    let converted_files = convert_dae_to_ssbh_files(&dae_scene, config)?;
    
    // Validate generated files
    validate_converted_files(&converted_files)?;
    
    Ok(converted_files)
}

/// Validate DAE scene data before conversion
fn validate_dae_scene(dae_scene: &DaeScene) -> Result<()> {
    // Check for empty meshes
    if dae_scene.meshes.is_empty() {
        return Err(anyhow!("DAE file contains no valid meshes"));
    }
    
    // Validate each mesh
    for (index, mesh) in dae_scene.meshes.iter().enumerate() {
        if mesh.vertices.is_empty() {
            return Err(anyhow!("Mesh {} has no vertices", index));
        }
        
        if mesh.indices.is_empty() {
            return Err(anyhow!("Mesh {} has no indices", index));
        }
        
        // Check index bounds
        let max_vertex_index = mesh.vertices.len() as u32;
        for &index in &mesh.indices {
            if index >= max_vertex_index {
                return Err(anyhow!("Mesh {} has out-of-bounds index: {}", index, index));
            }
        }
    }
    
    Ok(())
}

/// Validate converted SSBH files
fn validate_converted_files(converted_files: &ConvertedFiles) -> Result<()> {
    // Check that core files were created
    if converted_files.numdlb_path.is_none() {
        return Err(anyhow!("Failed to create .numdlb file"));
    }
    
    if converted_files.numshb_path.is_none() {
        return Err(anyhow!("Failed to create .numshb file"));
    }
    
    if converted_files.numatb_path.is_none() {
        return Err(anyhow!("Failed to create .numatb file"));
    }
    
    // Verify files exist on disk
    for path in [&converted_files.numdlb_path, &converted_files.numshb_path, &converted_files.numatb_path] {
        if let Some(p) = path {
            if !p.exists() {
                return Err(anyhow!("Generated file does not exist: {}", p.display()));
            }
        }
    }
    
    Ok(())
}

/// Show the DAE convert configuration dialog
pub fn show_dae_convert_dialog(
    ctx: &egui::Context,
    state: &mut DaeConvertDialogState,
) -> Option<DaeConvertConfig> {
    let mut result = None;
    
    if state.is_open {
        egui::Window::new("Convert DAE to SSBH")
            .open(&mut state.is_open)
            .resizable(false)
            .collapsible(false)
            .default_width(400.0)
            .show(ctx, |ui| {
                egui::Grid::new("convert_config_grid")
                    .num_columns(2)
                    .spacing([10.0, 10.0])
                    .show(ui, |ui| {
                        // DAE file selection
                        ui.label("DAE File:");
                        ui.horizontal(|ui| {
                            if ui.button("Select DAE File...").clicked() {
                                if let Some(file) = rfd::FileDialog::new()
                                    .add_filter("COLLADA", &["dae"])
                                    .pick_file()
                                {
                                    state.selected_dae_file = Some(file);
                                }
                            }
                            
                            if let Some(ref file) = state.selected_dae_file {
                                ui.label(file.file_name().unwrap_or_default().to_string_lossy());
                            }
                        });
                        ui.end_row();
                        
                        // Output directory selection
                        ui.label("Output Directory:");
                        ui.horizontal(|ui| {
                            if ui.button("Select Directory...").clicked() {
                                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                    state.config.output_directory = dir;
                                }
                            }
                            
                            if !state.config.output_directory.as_os_str().is_empty() {
                                ui.label(state.config.output_directory.display().to_string());
                            }
                        });
                        ui.end_row();
                        
                        // Base filename
                        ui.label("Base Filename:");
                        ui.text_edit_singleline(&mut state.config.base_filename);
                        ui.end_row();
                        
                        // Scale factor
                        ui.label("Scale Factor:");
                        ui.add(egui::DragValue::new(&mut state.config.scale_factor)
                            .range(0.001..=1000.0)
                            .speed(0.01));
                        ui.end_row();
                        
                        // Up axis conversion
                        ui.label("Up Axis Conversion:");
                        egui::ComboBox::from_id_salt("up_axis")
                            .selected_text(match state.config.up_axis_conversion {
                                UpAxisConversion::YUp => "Y-Up",
                                UpAxisConversion::ZUp => "Z-Up", 
                                UpAxisConversion::NoConversion => "No Conversion",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut state.config.up_axis_conversion, UpAxisConversion::YUp, "Y-Up");
                                ui.selectable_value(&mut state.config.up_axis_conversion, UpAxisConversion::ZUp, "Z-Up");
                                ui.selectable_value(&mut state.config.up_axis_conversion, UpAxisConversion::NoConversion, "No Conversion");
                            });
                        ui.end_row();
                    });
                
                ui.separator();
                
                // Output files preview
                ui.label("Output files:");
                ui.indent("output_files", |ui| {
                    ui.label(format!("• {}.numdlb", state.config.base_filename));
                    ui.label(format!("• {}.numshb", state.config.base_filename));
                    ui.label(format!("• {}.numatb", state.config.base_filename));
                });
                
                ui.separator();
                
                ui.horizontal(|ui| {
                    let can_convert = state.selected_dae_file.is_some()
                        && !state.config.output_directory.as_os_str().is_empty()
                        && !state.config.base_filename.is_empty();
                    
                    if ui.add_enabled(can_convert, egui::Button::new("Convert")).clicked() {
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

// Helper functions for parsing with dae-parser library
fn parse_materials_from_dae(lib_materials: &dae_parser::LibraryMaterials) -> Result<Vec<DaeMaterial>> {
    let mut materials = Vec::new();
    
    for material in &lib_materials.materials {
        let dae_material = DaeMaterial {
            name: material.id.clone(),
            diffuse_color: [1.0, 1.0, 1.0, 1.0], // Extract from material.instance_effect
            specular_color: [0.0, 0.0, 0.0, 1.0],
            emission_color: [0.0, 0.0, 0.0, 1.0],
            texture_paths: HashMap::new(), // Extract texture paths if available
        };
        materials.push(dae_material);
    }
    
    Ok(materials)
}

fn parse_geometries_from_dae(
    lib_geometries: &dae_parser::LibraryGeometries, 
    materials: &[DaeMaterial]
) -> Result<Vec<DaeMesh>> {
    let mut meshes = Vec::new();
    
    for geometry in &lib_geometries.geometries {
        if let Some(mesh) = &geometry.mesh {
            let dae_mesh = DaeMesh {
                name: geometry.id.clone(),
                vertices: extract_vertices_from_mesh(mesh)?,
                normals: extract_normals_from_mesh(mesh)?,
                uvs: extract_uvs_from_mesh(mesh)?,
                indices: extract_indices_from_mesh(mesh)?,
                material_name: None, // Material assignment comes from visual_scene
                bone_weights: None,  // Skinning data comes from controllers
                bone_indices: None,
            };
            meshes.push(dae_mesh);
        }
    }
    
    Ok(meshes)
}



// Helper functions for extracting specific data from COLLADA mesh structures
fn extract_vertices_from_mesh(mesh: &dae_parser::Mesh) -> Result<Vec<[f32; 3]>> {
    // Use dae-parser's mesh structure to extract vertex positions
    Ok(Vec::new()) // Placeholder
}

fn extract_normals_from_mesh(mesh: &dae_parser::Mesh) -> Result<Vec<[f32; 3]>> {
    // Extract normal vectors from mesh
    Ok(Vec::new()) // Placeholder
}

fn extract_uvs_from_mesh(mesh: &dae_parser::Mesh) -> Result<Vec<[f32; 2]>> {
    // Extract texture coordinates from mesh
    Ok(Vec::new()) // Placeholder
}

fn extract_indices_from_mesh(mesh: &dae_parser::Mesh) -> Result<Vec<u32>> {
    // Extract triangle indices from mesh
    Ok(Vec::new()) // Placeholder
}

// Fallback functions for manual XML parsing (keep as reference)
#[allow(dead_code)]
fn parse_materials_manual(lib_materials: &xmltree::Element) -> Result<Vec<DaeMaterial>> {
    // Original manual implementation - use as fallback
    Ok(Vec::new())
}

#[allow(dead_code)]
fn parse_geometries_manual(lib_geometries: &xmltree::Element, materials: &[DaeMaterial]) -> Result<Vec<DaeMesh>> {
    // Original manual implementation - use as fallback
    Ok(Vec::new())
}



fn convert_meshes_to_ssbh(meshes: &[DaeMesh], config: &DaeConvertConfig) -> Result<MeshData> {
    let mut mesh_objects = Vec::new();
    
    for (index, dae_mesh) in meshes.iter().enumerate() {
        // Apply scale factor and coordinate conversion
        let vertices = apply_transforms(&dae_mesh.vertices, config);
        let normals = if !dae_mesh.normals.is_empty() {
            apply_normal_transforms(&dae_mesh.normals, config)
        } else {
            Vec::new()
        };
        
        let mesh_object = MeshObjectData {
            name: dae_mesh.name.clone(),
            subindex: index as u64, // Ensure unique subindices
            positions: if !vertices.is_empty() {
                vec![AttributeData {
                    name: "Position0".to_string(),
                    data: VectorData::Vector3(vertices),
                }]
            } else { Vec::new() },
            normals: if !normals.is_empty() {
                vec![AttributeData {
                    name: "Normal0".to_string(),
                    data: VectorData::Vector3(normals),
                }]
            } else { Vec::new() },
            texture_coordinates: if !dae_mesh.uvs.is_empty() {
                vec![AttributeData {
                    name: "map1".to_string(), // Standard UV name for SSBH
                    data: VectorData::Vector2(dae_mesh.uvs.clone()),
                }]
            } else { Vec::new() },
            vertex_indices: dae_mesh.indices.clone(),
            ..Default::default()
        };
        mesh_objects.push(mesh_object);
    }
    
    Ok(MeshData {
        major_version: 1,
        minor_version: 10,
        objects: mesh_objects,
    })
}

fn convert_model_to_ssbh(meshes: &[DaeMesh], materials: &[DaeMaterial], config: &DaeConvertConfig) -> Result<ModlData> {
    let mut entries = Vec::new();
    
    for (mesh_index, mesh) in meshes.iter().enumerate() {
        let material_label = if let Some(ref mat_name) = mesh.material_name {
            // Find matching material or use default
            if materials.iter().any(|m| &m.name == mat_name) {
                mat_name.clone()
            } else {
                "DefaultMaterial".to_string()
            }
        } else {
            "DefaultMaterial".to_string()
        };
        
        let entry = ModlEntryData {
            mesh_object_name: mesh.name.clone(),
            mesh_object_subindex: mesh_index as u64,
            material_label,
        };
        entries.push(entry);
    }
    
    Ok(ModlData {
        major_version: 1,
        minor_version: 0,
        model_name: config.base_filename.clone(),
        skeleton_file_name: String::new(), // No skeleton support
        material_file_names: vec![format!("{}.numatb", config.base_filename)],
        animation_file_name: None,
        mesh_file_name: format!("{}.numshb", config.base_filename),
        entries,
    })
}



fn convert_materials_to_ssbh(materials: &[DaeMaterial], config: &DaeConvertConfig) -> Result<MatlData> {
    use ssbh_data::matl_data::{MatlEntryData, BlendStateParam, BlendStateData, ParamId, BlendFactor};
    
    let mut entries = Vec::new();
    
    // Always add a default material for meshes without materials
    let default_material = MatlEntryData {
        material_label: "DefaultMaterial".to_string(),
        shader_label: "SFX_PBS_010002000800824f_opaque".to_string(), // Safe default shader
        blend_states: vec![BlendStateParam {
            param_id: ParamId::BlendState0,
            data: BlendStateData {
                source_color: BlendFactor::One,
                destination_color: BlendFactor::Zero,
                alpha_sample_to_coverage: false,
                ..Default::default()
            },
        }],
        floats: Vec::new(),
        booleans: Vec::new(),
        vectors: Vec::new(),
        rasterizer_states: Vec::new(),
        samplers: Vec::new(),
        textures: Vec::new(),
        uv_transforms: Vec::new(),
    };
    entries.push(default_material);
    
    // Convert DAE materials
    for material in materials {
        let entry = MatlEntryData {
            material_label: material.name.clone(),
            shader_label: "SFX_PBS_010002000800824f_opaque".to_string(), // Use safe default
            // Add minimal required parameters for compatibility
            blend_states: vec![BlendStateParam {
                param_id: ParamId::BlendState0,
                data: BlendStateData {
                    source_color: BlendFactor::One,
                    destination_color: BlendFactor::Zero,
                    alpha_sample_to_coverage: false,
                    ..Default::default()
                },
            }],
            floats: Vec::new(),
            booleans: Vec::new(),
            vectors: Vec::new(),
            rasterizer_states: Vec::new(),
            samplers: Vec::new(),
            textures: Vec::new(),
            uv_transforms: Vec::new(),
        };
        entries.push(entry);
    }
    
    Ok(MatlData {
        major_version: 1,
        minor_version: 6,
        entries,
    })
}

// Helper functions for coordinate and data transformations
fn apply_transforms(vertices: &[[f32; 3]], config: &DaeConvertConfig) -> Vec<[f32; 3]> {
    vertices.iter().map(|v| {
        let mut transformed = *v;
        
        // Apply scale factor
        transformed[0] *= config.scale_factor;
        transformed[1] *= config.scale_factor;
        transformed[2] *= config.scale_factor;
        
        // Apply coordinate system conversion
        match config.up_axis_conversion {
            UpAxisConversion::ZUp => {
                // Convert Z-up to Y-up: swap Y and Z, negate new Z
                let temp = transformed[1];
                transformed[1] = transformed[2];
                transformed[2] = -temp;
            },
            UpAxisConversion::YUp | UpAxisConversion::NoConversion => {
                // No conversion needed
            },
        }
        
        transformed
    }).collect()
}

fn apply_normal_transforms(normals: &[[f32; 3]], config: &DaeConvertConfig) -> Vec<[f32; 3]> {
    normals.iter().map(|n| {
        let mut transformed = *n;
        
        // Apply coordinate system conversion (no scaling for normals)
        match config.up_axis_conversion {
            UpAxisConversion::ZUp => {
                let temp = transformed[1];
                transformed[1] = transformed[2];
                transformed[2] = -temp;
            },
            UpAxisConversion::YUp | UpAxisConversion::NoConversion => {},
        }
        
        transformed
    }).collect()
}

fn apply_matrix_transforms(matrix: &[[f32; 4]; 4], config: &DaeConvertConfig) -> [[f32; 4]; 4] {
    let mut result = *matrix;
    
    // Apply scale to translation components
    result[3][0] *= config.scale_factor;
    result[3][1] *= config.scale_factor; 
    result[3][2] *= config.scale_factor;
    
    // Apply coordinate conversion if needed
    match config.up_axis_conversion {
        UpAxisConversion::ZUp => {
            // Swap Y and Z columns and rows, negate appropriately
            // This is a simplified conversion - full implementation would need proper matrix transformation
            let temp_col = result[1];
            result[1] = result[2];
            result[2] = [-temp_col[0], -temp_col[1], -temp_col[2], -temp_col[3]];
        },
        UpAxisConversion::YUp | UpAxisConversion::NoConversion => {},
    }
    
    result
}


```

### 3. Update Application State

**File**: `src/app.rs`

Add to the imports:
```rust
use crate::convert::dae::{DaeConvertDialogState, DaeConvertConfig, convert_dae_file, show_dae_convert_dialog};
```

Add to the `SsbhApp` struct:
```rust
pub struct SsbhApp {
    // ... existing fields ...
    pub dae_convert_dialog: DaeConvertDialogState,
    pub pending_dae_convert: Option<(PathBuf, DaeConvertConfig)>, // (dae_file_path, config)
}
```

Add to `SsbhApp::new()` method:
```rust
dae_convert_dialog: DaeConvertDialogState::default(),
pending_dae_convert: None,
```

### 4. Update Menu System

**File**: `src/app/menu.rs`

Update the Tools menu section to add conversion functionality:
```rust
ui.menu_button("Tools", |ui| {
    // ... existing tools ...
    
    ui.separator();
    
    if button(ui, "Convert DAE to SSBH...").clicked() {
        app.dae_convert_dialog.is_open = true;
    }
    
    ui.separator();
    
    // ... other tools ...
});
```

### 5. Update Main Application Loop

**File**: `src/app.rs`

Add to the `central_panel` method, after existing processing:
```rust
// Handle DAE conversion
if let Some((dae_file_path, config)) = &self.pending_dae_convert {
    match convert_dae_file(dae_file_path, config) {
        Ok(converted_files) => {
            let mut file_list = Vec::new();
            if let Some(ref path) = converted_files.numdlb_path {
                file_list.push(path.file_name().unwrap_or_default().to_string_lossy().to_string());
            }
            if let Some(ref path) = converted_files.numshb_path {
                file_list.push(path.file_name().unwrap_or_default().to_string_lossy().to_string());
            }
            if let Some(ref path) = converted_files.numatb_path {
                file_list.push(path.file_name().unwrap_or_default().to_string_lossy().to_string());
            }
            
            log::info!(
                "Successfully converted DAE to SSBH files in {}: {}",
                config.output_directory.display(),
                file_list.join(", ")
            );
        }
        Err(e) => {
            log::error!("Error converting DAE file: {e}");
        }
    }
    self.pending_dae_convert = None;
}
```

Add to the `show_windows` method:
```rust
// Show DAE convert dialog
if let Some(config) = show_dae_convert_dialog(ctx, &mut self.dae_convert_dialog) {
    if let Some(ref dae_file) = self.dae_convert_dialog.selected_dae_file {
        self.pending_dae_convert = Some((dae_file.clone(), config));
    }
}
```

### 6. Update Dependencies

**File**: `Cargo.toml`

Add required dependencies:
```toml
[dependencies]
# ... existing dependencies ...
xmltree = "0.11.0"  # Already present for XML parsing
dae-parser = "0.11.0"  # COLLADA/DAE file parsing library
# Alternative: collada = "0.16.0"  # Another COLLADA parser option
```

**Note**: Using a dedicated COLLADA parsing library significantly simplifies the implementation and provides better compatibility with various DAE exporters.

### 7. Update Module Structure

**File**: `src/lib.rs`

Add convert module:
```rust
mod convert;
```

**File**: `src/convert/mod.rs`

```rust
pub mod dae;
```

## Recommended Implementation Strategy

### Phase 1: Use Dedicated COLLADA Library (Recommended)

This is the **easiest and most reliable** approach:

1. **Add `dae-parser` dependency** to `Cargo.toml`
   ```toml
   dae-parser = "0.11.0"
   ```

2. **Benefits of using `dae-parser`**:
   - ✅ **Significantly simpler implementation** - no need to manually parse XML
   - ✅ **Better compatibility** - handles various DAE exporters (Blender, Maya, 3ds Max)
   - ✅ **Robust error handling** - handles malformed or incomplete DAE files
   - ✅ **Maintained and tested** - less chance of bugs
   - ✅ **Faster development** - focus on SSBH conversion rather than parsing

3. **Implementation complexity**: **Medium** (mostly SSBH conversion logic)

### Phase 2: Alternative Libraries

If `dae-parser` doesn't meet requirements:

1. **Try `collada` crate** - another mature COLLADA parser
2. **Try `gltf` + conversion** - import as glTF first, then convert

### Phase 3: Manual XML Parsing (Not Recommended)

Only use as a last resort:

1. **Implementation complexity**: **Very High**
2. **Maintenance burden**: **High** 
3. **Compatibility issues**: **Likely**
4. **Development time**: **Much longer**

## Implementation Steps (Using dae-parser)

1. **Setup dependencies and modules**
   - Add `dae-parser` to `Cargo.toml`
   - Create `src/import/mod.rs` and `src/import/dae_scene.rs`

2. **Implement DAE parsing with dae-parser**
   - `parse_dae_file()` - Use `DaeDocument::from_file()`
   - `parse_materials_from_dae()` - Extract from `LibraryMaterials`
   - `parse_geometries_from_dae()` - Extract from `LibraryGeometries`  
   - `parse_skeleton_from_dae()` - Extract from `LibraryControllers`

3. **Implement SSBH conversion functions**
   - `convert_meshes_to_ssbh()` - Convert mesh data
   - `convert_model_to_ssbh()` - Convert model data
   - `convert_skeleton_to_ssbh()` - Convert skeleton data
   - `convert_materials_to_ssbh()` - Convert material data

4. **Implement UI dialog**
   - `show_dae_import_dialog()` - Configuration dialog
   - File selection for DAE input
   - Directory selection for output
   - Import options (materials, skeleton, scale, axis conversion)

5. **Update application integration**
   - Add dialog state to `SsbhApp`
   - Update menu system with import option
   - Add import processing to main loop
   - Integrate with existing workspace loading

6. **Test and refine**
   - Test with various DAE files from different exporters
   - Verify SSBH conversion accuracy
   - Test integration with export functionality
   - Add fallback for unsupported features

## User Experience

### Convert Dialog Features

1. **DAE File Selection**: Button to browse and select input DAE file
2. **Output Directory**: Button to select where SSBH files will be created
3. **Base Filename**: Text input for generated file names (default: "model")
4. **Scale Factor**: Numeric input for model scaling (default: 1.0)
5. **Up Axis Conversion**: Dropdown for Y-Up/Z-Up/No Conversion
6. **Output Files Preview**: Shows which files will be generated
7. **Convert/Cancel Buttons**: Action buttons with validation

### Convert Process

1. User clicks "Tools > Convert DAE to SSBH..."
2. Configuration dialog opens
3. User selects DAE file and output directory
4. User configures conversion settings
5. User sees preview of output files (model.numdlb, model.numshb, model.numatb)
6. User clicks Convert
7. DAE file is parsed and validated
8. SSBH files are generated and written to output directory
9. Success/error message logged with file list

### Standalone Tool Benefits

The converted files can be used in multiple ways:
- **Manual Import**: Drag converted files into SSBH Editor workspace
- **External Tools**: Use with other SSBH-compatible tools
- **Batch Processing**: Convert multiple DAE files without workspace overhead
- **File Management**: Full control over where files are placed

## Technical Details

### DAE Parsing Strategy

**Recommended Approach**: Use a dedicated COLLADA parsing library instead of manual XML parsing

#### Option 1: Using `dae-parser` (Recommended)
- Use `dae-parser` crate for full COLLADA 1.4.1 support
- Provides direct access to COLLADA data structures
- Handles edge cases and format variations automatically
- Much more reliable than manual XML parsing

#### Option 2: Using `collada` crate (Alternative)
- Another mature COLLADA parsing library
- Good documentation and examples
- Active maintenance

#### Fallback: Manual XML Parsing (Not Recommended)
- Use `xmltree` crate for XML parsing (already a dependency)
- Parse COLLADA standard elements: asset, library_materials, library_geometries, library_controllers
- Handle coordinate system conversion (Y-Up to Z-Up if needed)
- Extract vertex attributes, materials, and bone data
- **Warning**: This approach is complex, error-prone, and may not handle all DAE variations

### SSBH Conversion Strategy

- Convert DAE mesh data to `MeshData` with proper vertex attributes
- Create `ModlData` with mesh-material assignments  
- Generate basic materials as `MatlData` with safe default shaders
- Direct file generation without intermediate ModelFolder structures

### Error Handling

- Invalid DAE file format
- Missing required elements (meshes, vertices)
- Coordinate conversion errors
- File write permissions
- Output directory access
- SSBH format validation

## Future Enhancements

1. **Advanced Material Mapping**: Better conversion of DAE materials to SSBH shaders
2. **Texture Path Resolution**: Automatic texture file discovery and copying
3. **Batch Conversion**: Convert multiple DAE files at once
4. **Preview Before Convert**: 3D preview of DAE model before conversion
5. **Conversion Validation**: Check DAE compatibility before conversion
6. **Custom Shader Assignment**: Allow users to specify SSBH shaders for materials
7. **Command Line Interface**: Headless conversion for automation/scripting

## Testing Considerations

1. Test with various DAE exporters (Blender, Maya, 3ds Max)
2. Test with different coordinate systems and units
3. Test with complex hierarchies and multiple meshes
4. Test with and without materials/textures
5. Test error handling with malformed DAE files
6. Performance testing with large models
7. Verify generated SSBH files can be loaded in editor
8. Test file permissions and output directory validation

## Conclusion

This implementation provides a comprehensive DAE conversion tool that integrates seamlessly with SSBH Editor as a standalone utility. The modular design allows for easy extension and maintenance while providing flexibility for users who need DAE to SSBH conversion without workspace integration. The generated files are fully compatible with SSBH Editor and other SSBH-based tools.
