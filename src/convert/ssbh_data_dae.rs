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
        let normals = if !dae_mesh.normals.is_empty() {
            let transformed_normals = apply_normal_transforms(&dae_mesh.normals, config);
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
        
        // Convert bone influences using existing functionality
        let bone_influences = convert_dae_bone_influences_to_ssbh(&dae_mesh.bone_influences);
        
        // Construct MeshObjectData using direct construction (no factory methods available)
        let mesh_object = MeshObjectData {
            name: dae_mesh.name.clone(),
            subindex: index as u64,
            positions: vec![AttributeData {
                name: String::new(), // ssbh_data uses empty names for standard attributes
                data: VectorData::Vector3(vertices),
            }],
            normals: if !normals.is_empty() {
                vec![AttributeData {
                    name: String::new(),
                    data: VectorData::Vector3(normals),
                }]
            } else { Vec::new() },
            texture_coordinates: if !uvs.is_empty() {
                vec![AttributeData {
                    name: String::new(),
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
