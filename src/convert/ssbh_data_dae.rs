use anyhow::{Result, anyhow};
use ssbh_data::{
    mesh_data::{MeshData, MeshObjectData, AttributeData, VectorData},
    modl_data::{ModlData, ModlEntryData},
    skel_data::{SkelData, BoneData, BillboardType},
};
use std::path::Path;
use std::collections::HashSet;

// Re-use existing DAE parsing infrastructure
use super::dae::{
    DaeScene, DaeMesh, DaeBone, DaeConvertConfig, ConvertedFiles,
    parse_dae_file, validate_dae_scene, validate_converted_files,
    convert_dae_bone_influences_to_ssbh, apply_transforms, apply_normal_transforms
};

/// Convert DAE scene to SSBH files using proper ssbh_data integration
pub fn convert_dae_to_ssbh_files(
    dae_scene: &DaeScene,
    config: &DaeConvertConfig,
) -> Result<ConvertedFiles> {
    let mut converted_files = ConvertedFiles::default();
    
    // Generate skeleton from DAE bone hierarchy or mesh influences
    let skel_data = convert_skeleton_from_dae(&dae_scene.bones, &dae_scene.meshes, config)?;
    
    // Use proper ssbh_data construction with validation
    let mesh_data = convert_meshes_to_ssbh(&dae_scene.meshes, config)?;
    
    let modl_data = convert_model_to_ssbh(&dae_scene.meshes, config)?;
    
    // Write skeleton file first
    let skel_path = config.output_directory.join(format!("{}.nusktb", config.base_filename));
    skel_data.write_to_file(&skel_path)?;
    converted_files.nusktb_path = Some(skel_path);
    
    // Write mesh file
    let mesh_path = config.output_directory.join(format!("{}.numshb", config.base_filename));
    mesh_data.write_to_file(&mesh_path)?;
    converted_files.numshb_path = Some(mesh_path);
    
    // Write model file
    let modl_path = config.output_directory.join(format!("{}.numdlb", config.base_filename));
    modl_data.write_to_file(&modl_path)?;
    converted_files.numdlb_path = Some(modl_path);
    
    Ok(converted_files)
}

/// Convert DAE file to SSBH files using ssbh_data integration
pub fn convert_dae_file(
    dae_file_path: &Path,
    config: &DaeConvertConfig,
) -> Result<ConvertedFiles> {
    // Parse DAE file
    let dae_scene = parse_dae_file(dae_file_path)?;
    
    // Validate parsed data
    validate_dae_scene(&dae_scene)?;
    
    // Convert to SSBH files using proper ssbh_data integration
    let converted_files = convert_dae_to_ssbh_files(&dae_scene, config)?;
    
    // Validate generated files
    validate_converted_files(&converted_files)?;
    
    Ok(converted_files)
}

/// Convert DAE meshes to SSBH MeshData using proper ssbh_data construction
fn convert_meshes_to_ssbh(meshes: &[DaeMesh], config: &DaeConvertConfig) -> Result<MeshData> {
    let mut mesh_objects = Vec::new();
    
    for (index, dae_mesh) in meshes.iter().enumerate() {
        if dae_mesh.vertices.is_empty() {
            log::warn!("Skipping mesh '{}' with no vertices", dae_mesh.name);
            continue;
        }
        
        // Apply transformations and validate data consistency
        let vertices = apply_transforms(&dae_mesh.vertices, config);
        let vertex_count = vertices.len();
        
        let normals = if !dae_mesh.normals.is_empty() {
            let transformed_normals = apply_normal_transforms(&dae_mesh.normals, config);
            if transformed_normals.len() == vertex_count {
                transformed_normals
            } else {
                log::warn!(
                    "Mesh '{}': Normal count mismatch after transform. Expected {}, got {}. Generating default normals.",
                    dae_mesh.name, vertex_count, transformed_normals.len()
                );
                generate_default_normals(vertex_count)
            }
        } else {
            log::info!("Mesh '{}': No normals found, generating default normals.", dae_mesh.name);
            generate_default_normals(vertex_count)
        };
        
        // Validate UV data
        let uvs = if !dae_mesh.uvs.is_empty() {
            if dae_mesh.uvs.len() == vertex_count {
                dae_mesh.uvs.clone()
            } else {
                log::warn!(
                    "Mesh '{}': UV count mismatch. Expected {}, got {}. Generating default UVs.",
                    dae_mesh.name, vertex_count, dae_mesh.uvs.len()
                );
                generate_default_uvs(vertex_count)
            }
        } else {
            log::info!("Mesh '{}': No UVs found, generating default UVs.", dae_mesh.name);
            generate_default_uvs(vertex_count)
        };
        
        // Generate binormals (required for SSBH format)
        let binormals = generate_default_binormals(vertex_count);
        
        // Generate tangents (required for SSBH format)
        let tangents = generate_default_tangents(vertex_count);
        
        // Generate color sets (required for SSBH format)
        let color_sets = generate_default_color_sets(vertex_count);
        
        // Convert bone influences using existing functionality
        let bone_influences = convert_dae_bone_influences_to_ssbh(&dae_mesh.bone_influences);
        
        // Construct MeshObjectData with all required attributes
        let mesh_object = MeshObjectData {
            name: dae_mesh.name.clone(),
            subindex: index as u64,
            // Position0 - required
            positions: vec![AttributeData {
                name: "Position0".to_string(),
                data: VectorData::Vector3(vertices),
            }],
            // Normal0 - required
            normals: vec![AttributeData {
                name: "Normal0".to_string(),
                data: VectorData::Vector3(normals),
            }],
            // Binormal0 and Binormal1 - required (both with same data)
            binormals: vec![
                AttributeData {
                    name: "Binormal0".to_string(),
                    data: VectorData::Vector3(binormals.clone()),
                },
                AttributeData {
                    name: "Binormal1".to_string(),
                    data: VectorData::Vector3(binormals),
                },
            ],
            // Tangent0 and Tangent1 - required (both with same data)
            tangents: vec![
                AttributeData {
                    name: "Tangent0".to_string(),
                    data: VectorData::Vector4(tangents.clone()),
                },
                AttributeData {
                    name: "Tangent1".to_string(),
                    data: VectorData::Vector4(tangents),
                },
            ],
            // TextureCoordinate0 and HalfFloat2_0 - required
            texture_coordinates: vec![
                AttributeData {
                    name: "TextureCoordinate0".to_string(),
                    data: VectorData::Vector2(uvs.clone()),
                },
                AttributeData {
                    name: "HalfFloat2_0".to_string(),
                    data: VectorData::Vector2(uvs),
                },
            ],
            // colorSet1 - required
            color_sets: vec![AttributeData {
                name: "colorSet1".to_string(),
                data: VectorData::Vector4(color_sets),
            }],
            vertex_indices: dae_mesh.indices.clone(),
            bone_influences,
            ..Default::default()
        };
        
        log::info!(
            "Converted mesh '{}': {} vertices, {} normals, {} binormals, {} tangents, {} UVs, {} color sets, {} indices, {} bone influences",
            mesh_object.name,
            if let Some(pos_attr) = mesh_object.positions.first() {
                if let VectorData::Vector3(verts) = &pos_attr.data { verts.len() } else { 0 }
            } else { 0 },
            mesh_object.normals.len(),
            mesh_object.binormals.len(),
            mesh_object.tangents.len(),
            mesh_object.texture_coordinates.len(),
            mesh_object.color_sets.len(),
            mesh_object.vertex_indices.len(),
            mesh_object.bone_influences.len()
        );
        
        mesh_objects.push(mesh_object);
    }
    
    if mesh_objects.is_empty() {
        return Err(anyhow!("No valid mesh objects were created from DAE data"));
    }
    
    // Use standard ssbh_data construction with proper version numbers
    Ok(MeshData {
        major_version: 1,
        minor_version: 10,
        objects: mesh_objects,
    })
}

/// Convert DAE model to SSBH ModlData using proper ssbh_data construction
fn convert_model_to_ssbh(meshes: &[DaeMesh], config: &DaeConvertConfig) -> Result<ModlData> {
    let mut entries = Vec::new();
    
    for (mesh_index, mesh) in meshes.iter().enumerate() {
        // Use default material for all meshes since we don't generate .numatb
        let material_label = "DefaultMaterial".to_string();
        
        let entry = ModlEntryData {
            mesh_object_name: mesh.name.clone(),
            mesh_object_subindex: mesh_index as u64,
            material_label,
        };
        entries.push(entry);
    }
    
    // Use standard ssbh_data construction with proper file references
    Ok(ModlData {
        major_version: 1,
        minor_version: 0,
        model_name: config.base_filename.clone(),
        skeleton_file_name: format!("{}.nusktb", config.base_filename),
        material_file_names: vec![format!("{}.numatb", config.base_filename)], // Reference expected .numatb
        animation_file_name: None,
        mesh_file_name: format!("{}.numshb", config.base_filename),
        entries,
    })
}

/// Convert skeleton data from DAE bone hierarchy or mesh influences
fn convert_skeleton_from_dae(dae_bones: &[DaeBone], meshes: &[DaeMesh], _config: &DaeConvertConfig) -> Result<SkelData> {
    let mut bones = Vec::new();
    
    if !dae_bones.is_empty() {
        // Use bones from DAE hierarchy - this ensures ALL bones are included
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
        
        // Log bone names for debugging
        let bone_names: Vec<&str> = bones.iter().map(|b| b.name.as_str()).collect();
        log::info!("Bone names: {}", bone_names.join(", "));
    } else {
        // Fallback: collect bones from mesh influences (old behavior)
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
        
        log::warn!("No bone hierarchy found in DAE, falling back to mesh influences: {} bones", bones.len());
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
        log::info!("No bones found anywhere, created default root bone");
    }
    
    Ok(SkelData {
        major_version: 1,
        minor_version: 0,
        bones,
    })
}

/// Generate default normal vectors pointing up (0, 1, 0)
fn generate_default_normals(vertex_count: usize) -> Vec<[f32; 3]> {
    vec![[0.0, 1.0, 0.0]; vertex_count]
}

/// Generate default UV coordinates (0, 0) for all vertices
fn generate_default_uvs(vertex_count: usize) -> Vec<[f32; 2]> {
    vec![[0.0, 0.0]; vertex_count]
}

/// Generate default binormal vectors pointing right (1, 0, 0)
fn generate_default_binormals(vertex_count: usize) -> Vec<[f32; 3]> {
    vec![[1.0, 0.0, 0.0]; vertex_count]
}

/// Generate default tangent vectors pointing forward (0, 0, 1) with w=1 for handedness
fn generate_default_tangents(vertex_count: usize) -> Vec<[f32; 4]> {
    vec![[0.0, 0.0, 1.0, 1.0]; vertex_count]
}

/// Generate default color sets (white with full alpha)
fn generate_default_color_sets(vertex_count: usize) -> Vec<[f32; 4]> {
    vec![[1.0, 1.0, 1.0, 1.0]; vertex_count]
}
