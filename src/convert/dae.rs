use anyhow::{Result, anyhow};
use ssbh_data::{
    mesh_data::{MeshData, MeshObjectData, AttributeData, VectorData, BoneInfluence, VertexWeight},
    modl_data::{ModlData, ModlEntryData},
    matl_data::{MatlData, MatlEntryData, BlendStateParam, BlendStateData, ParamId, BlendFactor},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use xmltree::Element;

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
    pub bones: Vec<DaeBone>,
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
    pub bone_influences: Vec<DaeBoneInfluence>,
}

#[derive(Debug, Clone)]
pub struct DaeBoneInfluence {
    pub bone_name: String,
    pub vertex_weights: Vec<DaeVertexWeight>,
}

#[derive(Debug, Clone)]
pub struct DaeVertexWeight {
    pub vertex_index: u32,
    pub weight: f32,
}

#[derive(Debug, Clone)]
pub struct DaeBone {
    pub name: String,
    pub parent_index: Option<usize>,
    pub transform: [[f32; 4]; 4],
    pub inverse_bind_matrix: Option<[[f32; 4]; 4]>,
}

#[derive(Debug)]
pub struct DaeMaterial {
    pub name: String,
    pub diffuse_color: [f32; 4],
    pub specular_color: [f32; 4],
    pub emission_color: [f32; 4],
    pub texture_paths: HashMap<String, String>,
}

/// Parse DAE file and extract scene data using xmltree
pub fn parse_dae_file(file_path: &Path) -> Result<DaeScene> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| anyhow!("Failed to read DAE file: {}", e))?;
    
    let root = Element::parse(content.as_bytes())
        .map_err(|e| anyhow!("Failed to parse DAE XML: {}", e))?;
    
    let mut scene = DaeScene {
        meshes: Vec::new(),
        materials: Vec::new(),
        bones: Vec::new(),
        up_axis: UpAxisConversion::YUp,
    };
    
    // Extract up axis from asset information
    if let Some(asset) = find_child(&root, "asset") {
        if let Some(up_axis) = find_child(asset, "up_axis") {
            if let Some(text) = get_element_text(up_axis) {
                scene.up_axis = match text.as_str() {
                    "X_UP" => UpAxisConversion::NoConversion,
                    "Y_UP" => UpAxisConversion::YUp,
                    "Z_UP" => UpAxisConversion::ZUp,
                    _ => UpAxisConversion::YUp,
                };
            }
        }
    }
    
    // Parse materials
    if let Some(lib_materials) = find_child(&root, "library_materials") {
        scene.materials = parse_materials_from_xml(lib_materials)?;
    }
    
    // Parse geometries
    let mut geometry_id_to_name_map = std::collections::HashMap::new();
    if let Some(lib_geometries) = find_child(&root, "library_geometries") {
        scene.meshes = parse_geometries_from_xml(lib_geometries, &mut geometry_id_to_name_map)?;
    }
    
    // Parse controllers (bone influences and weights)
    if let Some(lib_controllers) = find_child(&root, "library_controllers") {
        parse_controllers_and_apply_to_meshes(lib_controllers, &mut scene.meshes, &geometry_id_to_name_map)?;
    }
    
    // Parse visual scenes for bone hierarchy
    if let Some(lib_visual_scenes) = find_child(&root, "library_visual_scenes") {
        scene.bones = parse_bone_hierarchy_from_visual_scenes(lib_visual_scenes)?;
    }
    
    // If no bones found in visual scenes, try library_nodes
    if scene.bones.is_empty() {
        if let Some(lib_nodes) = find_child(&root, "library_nodes") {
            scene.bones = parse_bone_hierarchy_from_nodes(lib_nodes)?;
        }
    }
    
    Ok(scene)
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
    
    // Convert and write skeleton data
    let skel_data = convert_skeleton_to_ssbh(&dae_scene.bones, &dae_scene.meshes, config)?;
    let skel_path = config.output_directory.join(format!("{}.nusktb", config.base_filename));
    skel_data.write_to_file(&skel_path)?;
    converted_files.nusktb_path = Some(skel_path);
    
    Ok(converted_files)
}

/// Result of DAE conversion operation
#[derive(Debug, Default)]
pub struct ConvertedFiles {
    pub numdlb_path: Option<PathBuf>,
    pub numshb_path: Option<PathBuf>,
    pub numatb_path: Option<PathBuf>,
    pub nusktb_path: Option<PathBuf>,
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
pub fn validate_dae_scene(dae_scene: &DaeScene) -> Result<()> {
    // Check for empty meshes
    if dae_scene.meshes.is_empty() {
        return Err(anyhow!("DAE file contains no valid meshes"));
    }
    
    // Validate each mesh
    for (index, mesh) in dae_scene.meshes.iter().enumerate() {
        if mesh.vertices.is_empty() {
            return Err(anyhow!("Mesh '{}' (index {}) has no vertices", mesh.name, index));
        }
        
        if mesh.indices.is_empty() {
            return Err(anyhow!("Mesh '{}' (index {}) has no indices", mesh.name, index));
        }
        
        // Check index bounds with more detailed error information
        let max_vertex_index = mesh.vertices.len() as u32;
        for (idx_pos, &index_val) in mesh.indices.iter().enumerate() {
            if index_val >= max_vertex_index {
                return Err(anyhow!(
                    "Mesh '{}' (index {}) has out-of-bounds index: {} at position {} (max valid index: {}, vertex count: {})",
                    mesh.name, 
                    index, 
                    index_val, 
                    idx_pos, 
                    max_vertex_index.saturating_sub(1), 
                    mesh.vertices.len()
                ));
            }
        }
        
        // Validate triangle count consistency
        if mesh.indices.len() % 3 != 0 {
            return Err(anyhow!(
                "Mesh '{}' (index {}) has invalid index count: {} (must be divisible by 3 for triangles)",
                mesh.name,
                index,
                mesh.indices.len()
            ));
        }
        
        // Check attribute data consistency
        if !mesh.normals.is_empty() && mesh.normals.len() != mesh.vertices.len() {
            log::warn!(
                "Mesh '{}': Normals count ({}) != vertices count ({}). This may cause issues.",
                mesh.name, mesh.normals.len(), mesh.vertices.len()
            );
        }
        
        if !mesh.uvs.is_empty() && mesh.uvs.len() != mesh.vertices.len() {
            log::warn!(
                "Mesh '{}': UV count ({}) != vertices count ({}). This may cause issues.",
                mesh.name, mesh.uvs.len(), mesh.vertices.len()
            );
        }
        
        // Validate bone influences
        for (bone_idx, bone_influence) in mesh.bone_influences.iter().enumerate() {
            for vertex_weight in &bone_influence.vertex_weights {
                if vertex_weight.vertex_index as usize >= mesh.vertices.len() {
                    log::warn!(
                        "Mesh '{}': Bone influence {} has vertex weight with invalid vertex index {} (max: {})",
                        mesh.name, bone_idx, vertex_weight.vertex_index, mesh.vertices.len() - 1
                    );
                }
                
                if vertex_weight.weight < 0.0 || vertex_weight.weight > 1.0 {
                    log::warn!(
                        "Mesh '{}': Bone influence {} has invalid weight {} (should be 0.0-1.0)",
                        mesh.name, bone_idx, vertex_weight.weight
                    );
                }
            }
        }
        
        // Log mesh statistics for debugging
        log::debug!(
            "Mesh '{}': {} vertices, {} indices ({} triangles), {} normals, {} UVs, {} bone influences",
            mesh.name,
            mesh.vertices.len(),
            mesh.indices.len(),
            mesh.indices.len() / 3,
            mesh.normals.len(),
            mesh.uvs.len(),
            mesh.bone_influences.len()
        );
    }
    
    Ok(())
}

/// Validate converted SSBH files
pub fn validate_converted_files(converted_files: &ConvertedFiles) -> Result<()> {
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
    
    if !state.is_open {
        return result;
    }
    
    let mut close_dialog = false;
    
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
                    ui.label(format!("• {}.nusktb", state.config.base_filename));
                });
                
                ui.separator();
                
                ui.horizontal(|ui| {
                    let can_convert = state.selected_dae_file.is_some()
                        && !state.config.output_directory.as_os_str().is_empty()
                        && !state.config.base_filename.is_empty();
                    
                    if ui.add_enabled(can_convert, egui::Button::new("Convert")).clicked() {
                        result = Some(state.config.clone());
                        close_dialog = true;
                    }
                    
                    if ui.button("Cancel").clicked() {
                        close_dialog = true;
                    }
                });
            });
    
    if close_dialog {
        state.is_open = false;
    }
    
    result
}

// Helper functions for parsing with xmltree
fn find_child<'a>(element: &'a Element, name: &str) -> Option<&'a Element> {
    element.children.iter().find_map(|node| {
        if let xmltree::XMLNode::Element(child) = node {
            if child.name == name {
                Some(child)
            } else {
                None
            }
        } else {
            None
        }
    })
}

fn get_element_text(element: &Element) -> Option<String> {
    element.children.iter().find_map(|node| {
        if let xmltree::XMLNode::Text(text) = node {
            Some(text.clone())
        } else {
            None
        }
    })
}

fn find_all_children<'a>(element: &'a Element, name: &str) -> Vec<&'a Element> {
    element.children.iter().filter_map(|node| {
        if let xmltree::XMLNode::Element(child) = node {
            if child.name == name {
                Some(child)
            } else {
                None
            }
        } else {
            None
        }
    }).collect()
}

fn parse_materials_from_xml(lib_materials: &Element) -> Result<Vec<DaeMaterial>> {
    let mut materials = Vec::new();
    
    for material_elem in find_all_children(lib_materials, "material") {
        if let Some(id) = material_elem.attributes.get("id") {
            let dae_material = DaeMaterial {
                name: id.clone(),
                diffuse_color: [1.0, 1.0, 1.0, 1.0],
                specular_color: [0.0, 0.0, 0.0, 1.0],
                emission_color: [0.0, 0.0, 0.0, 1.0],
                texture_paths: HashMap::new(),
            };
            materials.push(dae_material);
        }
    }
    
    Ok(materials)
}

fn parse_geometries_from_xml(lib_geometries: &Element, geometry_id_to_name_map: &mut HashMap<String, String>) -> Result<Vec<DaeMesh>> {
    let mut meshes = Vec::new();
    
    for geometry_elem in find_all_children(lib_geometries, "geometry") {
        if let Some(id) = geometry_elem.attributes.get("id") {
            if let Some(mesh_elem) = find_child(geometry_elem, "mesh") {
                // Use 'name' attribute if available, otherwise fall back to 'id'
                let mesh_name = geometry_elem.attributes.get("name")
                    .unwrap_or(id)
                    .clone();
                
                // Store the mapping from geometry id to mesh name
                geometry_id_to_name_map.insert(id.clone(), mesh_name.clone());
                
                let mut dae_mesh = DaeMesh {
                    name: mesh_name,
                    vertices: extract_vertices_from_xml_mesh(mesh_elem)?,
                    normals: extract_normals_from_xml_mesh(mesh_elem)?,
                    uvs: extract_uvs_from_xml_mesh(mesh_elem)?,
                    indices: extract_indices_from_xml_mesh(mesh_elem)?,
                    material_name: None,
                    bone_influences: Vec::new(),
                };
                
                // Post-process to ensure indices and vertex data are consistent
                optimize_mesh_data(&mut dae_mesh);
                
                meshes.push(dae_mesh);
            }
        }
    }
    
    Ok(meshes)
}

/// Parse controllers from DAE and apply bone influences to meshes
fn parse_controllers_and_apply_to_meshes(lib_controllers: &Element, meshes: &mut [DaeMesh], geometry_id_to_name_map: &HashMap<String, String>) -> Result<()> {
    for controller_elem in find_all_children(lib_controllers, "controller") {
        if let Some(controller_id) = controller_elem.attributes.get("id") {
            if let Some(skin_elem) = find_child(controller_elem, "skin") {
                if let Some(source_attr) = skin_elem.attributes.get("source") {
                    let geometry_id = source_attr.trim_start_matches('#');
                    
                    // Use the mapping to find the mesh name from geometry id
                    if let Some(mesh_name) = geometry_id_to_name_map.get(geometry_id) {
                        // Find the mesh that corresponds to this geometry
                        if let Some(mesh) = meshes.iter_mut().find(|m| &m.name == mesh_name) {
                            parse_skin_data_to_mesh(skin_elem, mesh)?;
                            log::info!(
                                "Applied bone influences from controller '{}' to mesh '{}' (geometry id: '{}')",
                                controller_id, mesh_name, geometry_id
                            );
                        } else {
                            log::warn!(
                                "Controller '{}' references mesh '{}' (geometry id: '{}') but mesh not found",
                                controller_id, mesh_name, geometry_id
                            );
                        }
                    } else {
                        log::warn!(
                            "Controller '{}' references unknown geometry id: '{}'",
                            controller_id, geometry_id
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

/// Parse skin data from DAE and convert to mesh bone influences
fn parse_skin_data_to_mesh(skin_elem: &Element, mesh: &mut DaeMesh) -> Result<()> {
    // Parse joints source
    let mut joint_names = Vec::new();
    let mut weights = Vec::new();
    
    // Find joints source
    for source_elem in find_all_children(skin_elem, "source") {
        if let Some(source_id) = source_elem.attributes.get("id") {
            if source_id.contains("joints") || source_id.contains("Joint") {
                if let Some(name_array) = find_child(source_elem, "Name_array") {
                    if let Some(names_text) = get_element_text(name_array) {
                        joint_names = names_text.split_whitespace().map(|s| s.to_string()).collect();
                    }
                }
            } else if source_id.contains("weights") || source_id.contains("Weight") {
                if let Some(float_array) = find_child(source_elem, "float_array") {
                    if let Some(weights_text) = get_element_text(float_array) {
                        weights = weights_text
                            .split_whitespace()
                            .filter_map(|s| s.parse::<f32>().ok())
                            .collect();
                    }
                }
            }
        }
    }
    
    if joint_names.is_empty() || weights.is_empty() {
        log::warn!("No valid joint names or weights found in skin data");
        return Ok(());
    }
    
    // Parse vertex weights
    if let Some(vertex_weights_elem) = find_child(skin_elem, "vertex_weights") {
        if let Some(count_attr) = vertex_weights_elem.attributes.get("count") {
            if let Ok(vertex_count) = count_attr.parse::<usize>() {
                parse_vertex_weights_data(vertex_weights_elem, mesh, &joint_names, &weights, vertex_count)?;
            }
        }
    }
    
    Ok(())
}

/// Parse vertex weights data and convert to bone influences
fn parse_vertex_weights_data(
    vertex_weights_elem: &Element,
    mesh: &mut DaeMesh,
    joint_names: &[String],
    weights: &[f32],
    vertex_count: usize,
) -> Result<()> {
    // Parse vcount (weights per vertex)
    let mut vcounts = Vec::new();
    if let Some(vcount_elem) = find_child(vertex_weights_elem, "vcount") {
        if let Some(vcount_text) = get_element_text(vcount_elem) {
            vcounts = vcount_text
                .split_whitespace()
                .filter_map(|s| s.parse::<usize>().ok())
                .collect();
        }
    }
    
    // Parse v (joint indices and weight indices)
    let mut v_data = Vec::new();
    if let Some(v_elem) = find_child(vertex_weights_elem, "v") {
        if let Some(v_text) = get_element_text(v_elem) {
            v_data = v_text
                .split_whitespace()
                .filter_map(|s| s.parse::<usize>().ok())
                .collect();
        }
    }
    
    if vcounts.len() != vertex_count {
        log::warn!(
            "Vertex weight count mismatch: expected {}, got {}",
            vertex_count, vcounts.len()
        );
        return Ok(());
    }
    
    // Group weights by bone
    let mut bone_influences: HashMap<String, Vec<DaeVertexWeight>> = HashMap::new();
    
    let mut v_index = 0;
    for (vertex_idx, &weight_count) in vcounts.iter().enumerate() {
        for _ in 0..weight_count {
            if v_index + 1 < v_data.len() {
                let joint_idx = v_data[v_index];
                let weight_idx = v_data[v_index + 1];
                
                if joint_idx < joint_names.len() && weight_idx < weights.len() {
                    let bone_name = &joint_names[joint_idx];
                    let weight = weights[weight_idx];
                    
                    // Only include non-zero weights
                    if weight > 0.0 {
                        bone_influences
                            .entry(bone_name.clone())
                            .or_insert_with(Vec::new)
                            .push(DaeVertexWeight {
                                vertex_index: vertex_idx as u32,
                                weight,
                            });
                    }
                }
                v_index += 2;
            }
        }
    }
    
    // Convert to mesh bone influences
    mesh.bone_influences = bone_influences
        .into_iter()
        .map(|(bone_name, vertex_weights)| DaeBoneInfluence {
            bone_name,
            vertex_weights,
        })
        .collect();
    
    log::debug!(
        "Parsed {} bone influences for mesh '{}'",
        mesh.bone_influences.len(),
        mesh.name
    );
    
    Ok(())
}

// Helper functions for extracting specific data from XML mesh structures
fn extract_vertices_from_xml_mesh(mesh_elem: &Element) -> Result<Vec<[f32; 3]>> {
    let mut vertices = Vec::new();
    
    // Find vertices element and position source
    if let Some(vertices_elem) = find_child(mesh_elem, "vertices") {
        if let Some(input_elem) = find_child(vertices_elem, "input") {
            if let Some(semantic) = input_elem.attributes.get("semantic") {
                if semantic == "POSITION" {
                    if let Some(source_ref) = input_elem.attributes.get("source") {
                        let source_id = source_ref.trim_start_matches('#');
                        
                        // Find the corresponding source
                        for source_elem in find_all_children(mesh_elem, "source") {
                            if let Some(id) = source_elem.attributes.get("id") {
                                if id == source_id {
                                    if let Some(float_array_elem) = find_child(source_elem, "float_array") {
                                        if let Some(data_text) = get_element_text(float_array_elem) {
                                            let values: Result<Vec<f32>, _> = data_text
                                                .split_whitespace()
                                                .map(|s| s.parse())
                                                .collect();
                                            
                                            if let Ok(values) = values {
                                                for chunk in values.chunks(3) {
                                                    if chunk.len() >= 3 {
                                                        vertices.push([chunk[0], chunk[1], chunk[2]]);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(vertices)
}

fn extract_normals_from_xml_mesh(mesh_elem: &Element) -> Result<Vec<[f32; 3]>> {
    let mut normals = Vec::new();
    
    // Look for normal data in triangles
    for triangles_elem in find_all_children(mesh_elem, "triangles") {
        for input_elem in find_all_children(triangles_elem, "input") {
            if let Some(semantic) = input_elem.attributes.get("semantic") {
                if semantic == "NORMAL" {
                    if let Some(source_ref) = input_elem.attributes.get("source") {
                        let source_id = source_ref.trim_start_matches('#');
                        
                        // Find the corresponding source
                        for source_elem in find_all_children(mesh_elem, "source") {
                            if let Some(id) = source_elem.attributes.get("id") {
                                if id == source_id {
                                    if let Some(float_array_elem) = find_child(source_elem, "float_array") {
                                        if let Some(data_text) = get_element_text(float_array_elem) {
                                            let values: Result<Vec<f32>, _> = data_text
                                                .split_whitespace()
                                                .map(|s| s.parse())
                                                .collect();
                                            
                                            if let Ok(values) = values {
                                                for chunk in values.chunks(3) {
                                                    if chunk.len() >= 3 {
                                                        normals.push([chunk[0], chunk[1], chunk[2]]);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }
        if !normals.is_empty() {
            break;
        }
    }
    
    Ok(normals)
}

fn extract_uvs_from_xml_mesh(mesh_elem: &Element) -> Result<Vec<[f32; 2]>> {
    let mut uvs = Vec::new();
    
    // Look for texture coordinate data in triangles
    for triangles_elem in find_all_children(mesh_elem, "triangles") {
        for input_elem in find_all_children(triangles_elem, "input") {
            if let Some(semantic) = input_elem.attributes.get("semantic") {
                if semantic == "TEXCOORD" {
                    if let Some(source_ref) = input_elem.attributes.get("source") {
                        let source_id = source_ref.trim_start_matches('#');
                        
                        // Find the corresponding source
                        for source_elem in find_all_children(mesh_elem, "source") {
                            if let Some(id) = source_elem.attributes.get("id") {
                                if id == source_id {
                                    if let Some(float_array_elem) = find_child(source_elem, "float_array") {
                                        if let Some(data_text) = get_element_text(float_array_elem) {
                                            let values: Result<Vec<f32>, _> = data_text
                                                .split_whitespace()
                                                .map(|s| s.parse())
                                                .collect();
                                            
                                            if let Ok(values) = values {
                                                for chunk in values.chunks(2) {
                                                    if chunk.len() >= 2 {
                                                        uvs.push([chunk[0], chunk[1]]);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    break;
                }
            }
        }
        if !uvs.is_empty() {
            break;
        }
    }
    
    Ok(uvs)
}

fn extract_indices_from_xml_mesh(mesh_elem: &Element) -> Result<Vec<u32>> {
    let mut indices = Vec::new();
    
    for triangles_elem in find_all_children(mesh_elem, "triangles") {
        // Get the stride (number of indices per vertex)
        let input_elements = find_all_children(triangles_elem, "input");
        let stride = input_elements.len();
        
        // Find the position input offset
        let mut position_offset = 0;
        for input_elem in &input_elements {
            if let Some(semantic) = input_elem.attributes.get("semantic") {
                if semantic == "VERTEX" || semantic == "POSITION" {
                    if let Some(offset_str) = input_elem.attributes.get("offset") {
                        position_offset = offset_str.parse().unwrap_or(0);
                    }
                    break;
                }
            }
        }
        
        if let Some(p_elem) = find_child(triangles_elem, "p") {
            if let Some(data_text) = get_element_text(p_elem) {
                let values: Result<Vec<u32>, _> = data_text
                    .split_whitespace()
                    .map(|s| s.parse())
                    .collect();
                
                if let Ok(values) = values {
                    if stride > 0 {
                        // Extract only the position indices using the correct offset and stride
                        for i in (position_offset..values.len()).step_by(stride) {
                            indices.push(values[i]);
                        }
                    } else {
                        // Fallback: if no stride detected, assume simple vertex indices
                        indices.extend(values);
                    }
                }
            }
        }
    }
    
    Ok(indices)
}

/// Optimize mesh data to ensure indices and vertex data are consistent
fn optimize_mesh_data(mesh: &mut DaeMesh) {
    if mesh.indices.is_empty() || mesh.vertices.is_empty() {
        return;
    }
    
    log::debug!(
        "Before optimization - Mesh '{}': {} vertices, {} normals, {} UVs, {} indices",
        mesh.name, mesh.vertices.len(), mesh.normals.len(), mesh.uvs.len(), mesh.indices.len()
    );
    
    // First, ensure all attribute data has consistent length with vertices
    align_attribute_data(mesh);
    
    // Find the maximum index used
    let max_index = mesh.indices.iter().max().copied().unwrap_or(0);
    let vertex_count = mesh.vertices.len() as u32;
    
    // If indices are within bounds and data is consistent, no further optimization needed
    if max_index < vertex_count {
        log::debug!(
            "Mesh '{}' data is already consistent - no optimization needed",
            mesh.name
        );
        return;
    }
    
    log::warn!(
        "Mesh '{}' has indices that exceed vertex count. Max index: {}, Vertex count: {}. Attempting to fix...",
        mesh.name, max_index, vertex_count
    );
    
    // Create a mapping of used indices to compact vertex data
    let mut used_indices: Vec<u32> = mesh.indices.iter().cloned().collect();
    used_indices.sort();
    used_indices.dedup();
    
    // Filter to only include valid indices
    used_indices.retain(|&idx| (idx as usize) < mesh.vertices.len());
    
    if used_indices.is_empty() {
        log::error!("No valid indices found for mesh '{}'", mesh.name);
        mesh.indices.clear();
        return;
    }
    
    // Create new vertex data using only referenced vertices
    let mut new_vertices = Vec::new();
    let mut new_normals = Vec::new();
    let mut new_uvs = Vec::new();
    let mut index_map = std::collections::HashMap::new();
    
    for (new_idx, &old_idx) in used_indices.iter().enumerate() {
        let old_idx_usize = old_idx as usize;
        if old_idx_usize < mesh.vertices.len() {
            new_vertices.push(mesh.vertices[old_idx_usize]);
            index_map.insert(old_idx, new_idx as u32);
            
            // Copy normals if available (should be same length as vertices now)
            if old_idx_usize < mesh.normals.len() {
                new_normals.push(mesh.normals[old_idx_usize]);
            }
            
            // Copy UVs if available (should be same length as vertices now)
            if old_idx_usize < mesh.uvs.len() {
                new_uvs.push(mesh.uvs[old_idx_usize]);
            }
        }
    }
    
    // Remap indices
    let mut new_indices = Vec::new();
    for &old_index in &mesh.indices {
        if let Some(&new_index) = index_map.get(&old_index) {
            new_indices.push(new_index);
        } else {
            log::warn!("Dropping invalid index {} in mesh '{}'", old_index, mesh.name);
        }
    }
    
    // Ensure we have triangles (index count divisible by 3)
    let remainder = new_indices.len() % 3;
    if remainder != 0 {
        new_indices.truncate(new_indices.len() - remainder);
        log::warn!("Truncated {} indices to maintain triangle integrity", remainder);
    }
    
    // Update mesh data
    mesh.vertices = new_vertices;
    mesh.normals = new_normals;
    mesh.uvs = new_uvs;
    mesh.indices = new_indices;
    
    log::info!(
        "Optimized mesh '{}': {} vertices, {} indices ({} triangles)",
        mesh.name,
        mesh.vertices.len(),
        mesh.indices.len(),
        mesh.indices.len() / 3
    );
}

/// Align attribute data to ensure all arrays have the same length as vertices
fn align_attribute_data(mesh: &mut DaeMesh) {
    let vertex_count = mesh.vertices.len();
    
    if vertex_count == 0 {
        mesh.normals.clear();
        mesh.uvs.clear();
        return;
    }
    
    // Align normals
    if !mesh.normals.is_empty() {
        if mesh.normals.len() < vertex_count {
            log::warn!(
                "Mesh '{}': Normals count ({}) < vertices count ({}). Padding with default normals.",
                mesh.name, mesh.normals.len(), vertex_count
            );
            // Pad with default normals (pointing up)
            mesh.normals.resize(vertex_count, [0.0, 1.0, 0.0]);
        } else if mesh.normals.len() > vertex_count {
            log::warn!(
                "Mesh '{}': Normals count ({}) > vertices count ({}). Truncating normals.",
                mesh.name, mesh.normals.len(), vertex_count
            );
            mesh.normals.truncate(vertex_count);
        }
    }
    
    // Align UVs
    if !mesh.uvs.is_empty() {
        if mesh.uvs.len() < vertex_count {
            log::warn!(
                "Mesh '{}': UV count ({}) < vertices count ({}). Padding with default UVs.",
                mesh.name, mesh.uvs.len(), vertex_count
            );
            // Pad with default UVs
            mesh.uvs.resize(vertex_count, [0.0, 0.0]);
        } else if mesh.uvs.len() > vertex_count {
            log::warn!(
                "Mesh '{}': UV count ({}) > vertices count ({}). Truncating UVs.",
                mesh.name, mesh.uvs.len(), vertex_count
            );
            mesh.uvs.truncate(vertex_count);
        }
    }
    
    log::debug!(
        "Aligned attribute data for mesh '{}': {} vertices, {} normals, {} UVs",
        mesh.name, mesh.vertices.len(), mesh.normals.len(), mesh.uvs.len()
    );
}

fn convert_meshes_to_ssbh(meshes: &[DaeMesh], config: &DaeConvertConfig) -> Result<MeshData> {
    let mut mesh_objects = Vec::new();
    
    for (index, dae_mesh) in meshes.iter().enumerate() {
        // Validate data consistency before conversion
        if dae_mesh.vertices.is_empty() {
            log::warn!("Skipping mesh '{}' with no vertices", dae_mesh.name);
            continue;
        }
        
        // Apply scale factor and coordinate conversion
        let vertices = apply_transforms(&dae_mesh.vertices, config);
        let normals = if !dae_mesh.normals.is_empty() {
            let transformed_normals = apply_normal_transforms(&dae_mesh.normals, config);
            // Ensure normals match vertex count
            if transformed_normals.len() == vertices.len() {
                transformed_normals
            } else {
                log::warn!(
                    "Mesh '{}': Normal count mismatch after transform. Expected {}, got {}. Skipping normals.",
                    dae_mesh.name, vertices.len(), transformed_normals.len()
                );
                Vec::new()
            }
        } else {
            Vec::new()
        };
        
        // Validate UV data
        let uvs = if !dae_mesh.uvs.is_empty() {
            if dae_mesh.uvs.len() == vertices.len() {
                dae_mesh.uvs.clone()
            } else {
                log::warn!(
                    "Mesh '{}': UV count mismatch. Expected {}, got {}. Skipping UVs.",
                    dae_mesh.name, vertices.len(), dae_mesh.uvs.len()
                );
                Vec::new()
            }
        } else {
            Vec::new()
        };
        
        // Convert DAE bone influences to SSBH bone influences
        let bone_influences = convert_dae_bone_influences_to_ssbh(&dae_mesh.bone_influences);
        
        let mesh_object = MeshObjectData {
            name: dae_mesh.name.clone(),
            subindex: index as u64,
            positions: vec![AttributeData {
                //debug: removed string attribute names for numshb export
                name: String::new(), // "Position0".to_string(),
                data: VectorData::Vector3(vertices),
            }],
            normals: if !normals.is_empty() {
                vec![AttributeData {
                    //debug: removed string attribute names for numshb export
                    name: String::new(), // "Normal0".to_string(),
                    data: VectorData::Vector3(normals),
                }]
            } else { Vec::new() },
            texture_coordinates: if !uvs.is_empty() {
                vec![AttributeData {
                    //debug: removed string attribute names for numshb export
                    name: String::new(), // "map1".to_string(),
                    data: VectorData::Vector2(uvs),
                }]
            } else { Vec::new() },
            vertex_indices: dae_mesh.indices.clone(),
            bone_influences,
            ..Default::default()
        };
        
        log::info!(
            "Converted mesh '{}': {} vertices, {} normals, {} UVs, {} indices, {} bone influences",
            mesh_object.name,
            if let Some(pos_attr) = mesh_object.positions.first() {
                if let VectorData::Vector3(verts) = &pos_attr.data { verts.len() } else { 0 }
            } else { 0 },
            if let Some(norm_attr) = mesh_object.normals.first() {
                if let VectorData::Vector3(norms) = &norm_attr.data { norms.len() } else { 0 }
            } else { 0 },
            if let Some(uv_attr) = mesh_object.texture_coordinates.first() {
                if let VectorData::Vector2(uvs) = &uv_attr.data { uvs.len() } else { 0 }
            } else { 0 },
            mesh_object.vertex_indices.len(),
            mesh_object.bone_influences.len()
        );
        
        mesh_objects.push(mesh_object);
    }
    
    if mesh_objects.is_empty() {
        return Err(anyhow!("No valid mesh objects were created from DAE data"));
    }
    
    Ok(MeshData {
        major_version: 1,
        minor_version: 10,
        objects: mesh_objects,
        is_vs2: true,
    })
}

fn convert_model_to_ssbh(meshes: &[DaeMesh], materials: &[DaeMaterial], config: &DaeConvertConfig) -> Result<ModlData> {
    let mut entries = Vec::new();
    
    for (mesh_index, mesh) in meshes.iter().enumerate() {
        let material_label = if let Some(ref mat_name) = mesh.material_name {
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
        skeleton_file_name: format!("{}.nusktb", config.base_filename),
        material_file_names: vec![format!("{}.numatb", config.base_filename)],
        animation_file_name: None,
        mesh_file_name: format!("{}.numshb", config.base_filename),
        entries,
    })
}

fn convert_materials_to_ssbh(materials: &[DaeMaterial], _config: &DaeConvertConfig) -> Result<MatlData> {
    let mut entries = Vec::new();
    
    // Always add a default material for meshes without materials
    let default_material = MatlEntryData {
        material_label: "DefaultMaterial".to_string(),
        shader_label: "SFX_PBS_010002000800824f_opaque".to_string(),
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
            shader_label: "SFX_PBS_010002000800824f_opaque".to_string(),
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

/// Convert DAE bone influences to SSBH bone influences
pub fn convert_dae_bone_influences_to_ssbh(dae_influences: &[DaeBoneInfluence]) -> Vec<BoneInfluence> {
    let mut ssbh_influences = Vec::new();
    
    for dae_influence in dae_influences {
        let vertex_weights: Vec<VertexWeight> = dae_influence
            .vertex_weights
            .iter()
            .map(|dae_weight| VertexWeight {
                vertex_index: dae_weight.vertex_index,
                vertex_weight: dae_weight.weight,
            })
            .collect();
        
        if !vertex_weights.is_empty() {
            ssbh_influences.push(BoneInfluence {
                bone_name: dae_influence.bone_name.clone(),
                vertex_weights,
            });
        }
    }
    
    log::debug!("Converted {} bone influences to SSBH format", ssbh_influences.len());
    ssbh_influences
}

// Helper functions for coordinate and data transformations
pub fn apply_transforms(vertices: &[[f32; 3]], config: &DaeConvertConfig) -> Vec<[f32; 3]> {
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

pub fn apply_normal_transforms(normals: &[[f32; 3]], config: &DaeConvertConfig) -> Vec<[f32; 3]> {
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

/// Parse bone hierarchy from library_visual_scenes
fn parse_bone_hierarchy_from_visual_scenes(lib_visual_scenes: &Element) -> Result<Vec<DaeBone>> {
    let mut bones = Vec::new();
    
    for visual_scene in find_all_children(lib_visual_scenes, "visual_scene") {
        for node in find_all_children(visual_scene, "node") {
            parse_node_hierarchy(node, None, &mut bones)?;
        }
    }
    
    if !bones.is_empty() {
        let bone_names: Vec<&str> = bones.iter().map(|b| b.name.as_str()).collect();
        log::info!("Parsed {} bones from visual scenes: {}", bones.len(), bone_names.join(", "));
    } else {
        log::info!("No bones found in visual scenes");
    }
    Ok(bones)
}

/// Parse bone hierarchy from library_nodes
fn parse_bone_hierarchy_from_nodes(lib_nodes: &Element) -> Result<Vec<DaeBone>> {
    let mut bones = Vec::new();
    
    for node in find_all_children(lib_nodes, "node") {
        parse_node_hierarchy(node, None, &mut bones)?;
    }
    
    if !bones.is_empty() {
        let bone_names: Vec<&str> = bones.iter().map(|b| b.name.as_str()).collect();
        log::info!("Parsed {} bones from nodes: {}", bones.len(), bone_names.join(", "));
    } else {
        log::info!("No bones found in nodes");
    }
    Ok(bones)
}

/// Recursively parse node hierarchy to extract bone information
fn parse_node_hierarchy(
    node: &Element,
    parent_index: Option<usize>,
    bones: &mut Vec<DaeBone>,
) -> Result<()> {
    if let Some(node_id) = node.attributes.get("id") {
        // Check if this is a bone/joint node
        let node_type = node.attributes.get("type").map(|s| s.as_str()).unwrap_or("");
        let node_name = node.attributes.get("name").unwrap_or(node_id);
        let node_sid = node.attributes.get("sid").map(|s| s.as_str()).unwrap_or("");
        
        let is_bone = node_type == "JOINT" || 
                     node_id.to_lowercase().contains("bone") || 
                     node_id.to_lowercase().contains("joint") ||
                     node_name.to_lowercase().contains("bone") ||
                     node_name.to_lowercase().contains("joint") ||
                     node_sid.to_lowercase().contains("bone") ||
                     node_sid.to_lowercase().contains("joint");
        
        if is_bone || parent_index.is_some() {
            // Use 'name' attribute if available, otherwise fall back to 'id'
            let bone_name = node.attributes.get("name")
                .or_else(|| node.attributes.get("sid"))
                .unwrap_or(node_id)
                .clone();
            
            // Parse transformation matrix
            let transform = parse_node_transform(node);
            
            let bone = DaeBone {
                name: bone_name.clone(),
                parent_index,
                transform,
                inverse_bind_matrix: None,
            };
            
            let current_index = bones.len();
            log::debug!("Parsed bone: '{}' (id: '{}', type: '{}')", bone_name, node_id, node_type);
            bones.push(bone);
            
            // Recursively parse child nodes
            for child_node in find_all_children(node, "node") {
                parse_node_hierarchy(child_node, Some(current_index), bones)?;
            }
        } else {
            // Not a bone, but check children anyway
            for child_node in find_all_children(node, "node") {
                parse_node_hierarchy(child_node, parent_index, bones)?;
            }
        }
    }
    
    Ok(())
}

/// Parse transformation matrix from a node
fn parse_node_transform(node: &Element) -> [[f32; 4]; 4] {
    // Look for matrix element first
    if let Some(matrix_elem) = find_child(node, "matrix") {
        if let Some(matrix_text) = get_element_text(matrix_elem) {
            if let Ok(values) = parse_matrix_values(&matrix_text) {
                if values.len() >= 16 {
                    return [
                        [values[0], values[1], values[2], values[3]],
                        [values[4], values[5], values[6], values[7]],
                        [values[8], values[9], values[10], values[11]],
                        [values[12], values[13], values[14], values[15]],
                    ];
                }
            }
        }
    }
    
    // If no matrix, try to build from translate, rotate, scale
    let mut transform = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    
    // Apply translation
    if let Some(translate_elem) = find_child(node, "translate") {
        if let Some(translate_text) = get_element_text(translate_elem) {
            if let Ok(values) = parse_matrix_values(&translate_text) {
                if values.len() >= 3 {
                    transform[0][3] = values[0];
                    transform[1][3] = values[1];
                    transform[2][3] = values[2];
                }
            }
        }
    }
    
    // Note: For full accuracy, we should also handle rotation and scale,
    // but identity matrix is sufficient for basic skeleton structure
    
    transform
}

/// Parse matrix values from text
fn parse_matrix_values(text: &str) -> Result<Vec<f32>> {
    text.split_whitespace()
        .map(|s| s.parse::<f32>().map_err(|e| anyhow!("Failed to parse float: {}", e)))
        .collect()
}

/// Convert DAE bones to SSBH skeleton data
fn convert_skeleton_to_ssbh(
    dae_bones: &[DaeBone],
    meshes: &[DaeMesh],
    _config: &DaeConvertConfig,
) -> Result<ssbh_data::skel_data::SkelData> {
    use ssbh_data::skel_data::{SkelData, BoneData, BillboardType};
    use std::collections::HashSet;
    
    let mut bones = Vec::new();
    
    if !dae_bones.is_empty() {
        // Use bones from DAE hierarchy
        for dae_bone in dae_bones {
            let bone_data = BoneData {
                name: dae_bone.name.clone(),
                transform: dae_bone.transform,
                parent_index: dae_bone.parent_index,
                billboard_type: BillboardType::Disabled,
            };
            bones.push(bone_data);
        }
        
        log::info!("Created skeleton with {} bones from DAE hierarchy", bones.len());
    } else {
        // Fallback: collect bones from mesh influences
        let mut bone_names = HashSet::new();
        for mesh in meshes {
            for bone_influence in &mesh.bone_influences {
                bone_names.insert(bone_influence.bone_name.clone());
            }
        }
        
        let mut bone_names: Vec<String> = bone_names.into_iter().collect();
        bone_names.sort();
        
        for (index, bone_name) in bone_names.iter().enumerate() {
            let bone_data = BoneData {
                name: bone_name.clone(),
                transform: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
                parent_index: if index == 0 { None } else { Some(index - 1) },
                billboard_type: BillboardType::Disabled,
            };
            bones.push(bone_data);
        }
        
        log::info!("Created skeleton with {} bones from mesh influences", bones.len());
    }
    
    // If still no bones found, create a default root bone
    if bones.is_empty() {
        let root_bone = BoneData {
            name: "Root".to_string(),
            transform: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            parent_index: None,
            billboard_type: BillboardType::Disabled,
        };
        bones.push(root_bone);
        log::info!("No bones found, created default root bone");
    }
    
    Ok(SkelData {
        major_version: 1,
        minor_version: 0,
        bones,
    })
}
