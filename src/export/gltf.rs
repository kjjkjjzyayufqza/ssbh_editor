use anyhow::Result;
use gltf_json::{accessor, buffer, material, mesh, scene, validation, Accessor, Asset, Buffer, Index, Material, Mesh, Node, Root, Scene, Skin};
use gltf_json::buffer::View as BufferView;
use ssbh_data::{mesh_data::MeshData, skel_data::SkelData};
use ssbh_wgpu::ModelFolder;
use std::{collections::BTreeMap, path::Path, fs};

/// Export a model folder to GLTF format
/// If use_json_files is true, load data from model.json and skeleton.json in root directory instead
pub fn export_scene_to_gltf(
    model_folder: &ModelFolder,
    output_path: &Path,
    use_json_files: bool,
) -> Result<()> {
    // If using JSON files, load data from root directory JSON files
    let effective_model_folder = if use_json_files {
        &load_model_from_json_files()?
    } else {
        model_folder
    };

    let mut gltf_root = Root {
        asset: Asset {
            generator: Some("SSBH Editor".to_string()),
            version: "2.0".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    let mut buffer_data = Vec::new();
    let mut accessors = Vec::new();
    let mut buffer_views = Vec::new();
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();
    let mut materials = Vec::new();
    let mut skins = Vec::new();

    // Process skeleton data first to establish bone hierarchy
    let skeleton_node_count = if let Some((_, Some(skel_data))) = effective_model_folder.skels.first() {
        process_skeleton_data(
            skel_data,
            &mut nodes,
        )?;
        skel_data.bones.len()
    } else {
        0
    };

    // Process mesh data and create mesh nodes
    if let Some((_, Some(mesh_data))) = effective_model_folder.meshes.first() {
        let skel_data = effective_model_folder.skels.first()
            .and_then(|(_, skel)| skel.as_ref());
        
        let mesh_count = process_mesh_data(
            mesh_data,
            skel_data,
            &mut gltf_root,
            &mut buffer_data,
            &mut buffer_views,
            &mut accessors,
            &mut meshes,
            &mut materials,
        )?;
        
        // Create skin object if skeleton exists and has bone influences
        let skin_index = if let Some(skel_data) = skel_data {
            if mesh_data.objects.iter().any(|obj| !obj.bone_influences.is_empty()) {
                let skin_index = create_skin(
                    skel_data,
                    &mut buffer_data,
                    &mut buffer_views,
                    &mut accessors,
                    &mut skins,
                )?;
                Some(skin_index)
            } else {
                None
            }
        } else {
            None
        };
        
        // Create mesh nodes that reference the meshes
        create_mesh_nodes(
            mesh_count,
            skeleton_node_count,
            skin_index,
            &mut nodes,
        );
    }

    // Create buffer
    if !buffer_data.is_empty() {
        let buffer = Buffer {
            byte_length: validation::USize64::from(buffer_data.len()),
            uri: Some("scene.bin".to_string()),
            name: None,
            extensions: Default::default(),
            extras: Default::default(),
        };
        gltf_root.buffers = vec![buffer];
    }

    // Create buffer views
    if !buffer_views.is_empty() {
        gltf_root.buffer_views = buffer_views;
    }

    // Set accessors, meshes, materials, nodes, and skins
    gltf_root.accessors = accessors;
    gltf_root.meshes = meshes;
    gltf_root.materials = materials;
    gltf_root.nodes = nodes;
    if !skins.is_empty() {
        gltf_root.skins = skins;
    }

    // Create scene with root nodes (nodes without parents)
    // For skeleton nodes, only include root bones (those without parent_index)
    // For mesh nodes, include all mesh nodes
    let mut scene_nodes = Vec::new();
    
    // Add root skeleton nodes (if any skeleton exists)
    if let Some((_, Some(skel_data))) = effective_model_folder.skels.first() {
        for (bone_index, bone) in skel_data.bones.iter().enumerate() {
            if bone.parent_index.is_none() {
                scene_nodes.push(Index::new(bone_index as u32));
            }
        }
    }

    // Add all mesh nodes (they start after skeleton nodes)
    let skeleton_count = effective_model_folder.skels.first()
        .and_then(|(_, skel)| skel.as_ref())
        .map_or(0, |s| s.bones.len());
    
    for mesh_node_index in skeleton_count..gltf_root.nodes.len() {
        scene_nodes.push(Index::new(mesh_node_index as u32));
    }
    
    let scene = Scene {
        name: None,
        nodes: scene_nodes,
        extensions: Default::default(),
        extras: Default::default(),
    };
    gltf_root.scenes = vec![scene];
    gltf_root.scene = Some(Index::new(0));

    // Write GLTF file
    let gltf_json = serde_json::to_string_pretty(&gltf_root)?;
    std::fs::write(output_path.with_extension("gltf"), gltf_json)?;

    // Write binary buffer if exists
    if !buffer_data.is_empty() {
        let bin_path = output_path.with_file_name("scene.bin");
        std::fs::write(bin_path, buffer_data)?;
    }

    Ok(())
}

fn process_mesh_data(
    mesh_data: &MeshData,
    skel_data: Option<&SkelData>,
    _gltf_root: &mut Root,
    buffer_data: &mut Vec<u8>,
    buffer_views: &mut Vec<BufferView>,
    accessors: &mut Vec<Accessor>,
    meshes: &mut Vec<Mesh>,
    materials: &mut Vec<Material>,
) -> Result<usize> {
    // Create a default material
    let default_material = Material {
        name: Some("DefaultMaterial".to_string()),
        pbr_metallic_roughness: material::PbrMetallicRoughness {
            base_color_factor: material::PbrBaseColorFactor([1.0, 1.0, 1.0, 1.0]),
            metallic_factor: material::StrengthFactor(0.0),
            roughness_factor: material::StrengthFactor(1.0),
            base_color_texture: None,
            metallic_roughness_texture: None,
            extensions: Default::default(),
            extras: Default::default(),
        },
        alpha_cutoff: None,
        alpha_mode: validation::Checked::Valid(material::AlphaMode::Opaque),
        double_sided: false,
        normal_texture: None,
        occlusion_texture: None,
        emissive_texture: None,
        emissive_factor: material::EmissiveFactor([0.0, 0.0, 0.0]),
        extensions: Default::default(),
        extras: Default::default(),
    };
    materials.push(default_material);

    // Process each mesh object
    for mesh_object in &mesh_data.objects {
        if mesh_object.positions.is_empty() {
            continue;
        }

        let position_data = &mesh_object.positions[0].data;
        let mut primitives = Vec::new();

        // Create position accessor
        let position_vec3_data = convert_vector_data_to_vec3(position_data)?;
        let position_accessor_index = create_vec3_accessor(
            &position_vec3_data,
            buffer_data,
            buffer_views,
            accessors,
            "POSITION",
        )?;

        // Create normal accessor if available
        let normal_accessor_index = if !mesh_object.normals.is_empty() {
            let normal_vec3_data = convert_vector_data_to_vec3(&mesh_object.normals[0].data)?;
            Some(create_vec3_accessor(
                &normal_vec3_data,
                buffer_data,
                buffer_views,
                accessors,
                "NORMAL",
            )?)
        } else {
            None
        };

        // Create texture coordinate accessor if available
        let texcoord_accessor_index = if !mesh_object.texture_coordinates.is_empty() {
            let texcoord_vec2_data = convert_vector_data_to_vec2(&mesh_object.texture_coordinates[0].data)?;
            Some(create_vec2_accessor(
                &texcoord_vec2_data,
                buffer_data,
                buffer_views,
                accessors,
                "TEXCOORD_0",
            )?)
        } else {
            None
        };

        // Create joint and weight accessors if bone influences exist
        let (joints_accessor_index, weights_accessor_index) = if !mesh_object.bone_influences.is_empty() && skel_data.is_some() {
            let skel = skel_data.unwrap();
            let vertex_count = mesh_object.vertex_count().unwrap_or(0) as usize;
            
            // Create joint and weight data for each vertex (up to 4 influences per vertex)
            let mut joints_data = vec![[0u16; 4]; vertex_count];
            let mut weights_data = vec![[0.0f32; 4]; vertex_count];
            
            // Build bone name to index mapping
            let bone_name_to_index: BTreeMap<String, u16> = skel.bones
                .iter()
                .enumerate()
                .filter_map(|(i, bone)| {
                    if i <= u16::MAX as usize {
                        Some((bone.name.clone(), i as u16))
                    } else {
                        eprintln!("Warning: Bone index {} exceeds u16::MAX, skipping bone '{}'", i, bone.name);
                        None
                    }
                })
                .collect();
            
            // Collect all influences per vertex first
            let mut vertex_influences: Vec<Vec<(u16, f32)>> = vec![Vec::new(); vertex_count];
            
            for influence in &mesh_object.bone_influences {
                if let Some(&bone_index) = bone_name_to_index.get(&influence.bone_name) {
                    for vertex_weight in &influence.vertex_weights {
                        let vertex_idx = vertex_weight.vertex_index as usize;
                        if vertex_idx < vertex_count {
                            vertex_influences[vertex_idx].push((bone_index, vertex_weight.vertex_weight));
                        }
                    }
                }
            }
            
            // Process each vertex: sort by weight and take top 4, then normalize
            for (vertex_idx, influences) in vertex_influences.iter_mut().enumerate() {
                if influences.is_empty() {
                    // For vertices with no influences, assign to root bone with weight 1.0
                    // This prevents rendering issues with unweighted vertices
                    joints_data[vertex_idx][0] = 0; // Assign to first bone (usually root)
                    weights_data[vertex_idx][0] = 1.0;
                    continue;
                }
                
                // Sort by weight descending and take top 4
                influences.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                influences.truncate(4);
                
                // Calculate total weight for normalization
                let total_weight: f32 = influences.iter().map(|(_, weight)| weight).sum();
                
                if total_weight > 0.0 {
                    // Normalize weights and assign to arrays
                    for (slot, &(joint_index, weight)) in influences.iter().enumerate() {
                        joints_data[vertex_idx][slot] = joint_index;
                        weights_data[vertex_idx][slot] = weight / total_weight;
                    }
                } else {
                    // If total weight is zero, assign to root bone with weight 1.0
                    joints_data[vertex_idx][0] = 0;
                    weights_data[vertex_idx][0] = 1.0;
                }
            }
            
            let joints_accessor = create_joints_accessor(
                &joints_data,
                buffer_data,
                buffer_views,
                accessors,
            )?;
            
            let weights_accessor = create_weights_accessor(
                &weights_data,
                buffer_data,
                buffer_views,
                accessors,
            )?;
            
            (Some(joints_accessor), Some(weights_accessor))
        } else {
            (None, None)
        };

        // Create indices accessor
        let indices_accessor_index = if !mesh_object.vertex_indices.is_empty() {
            Some(create_indices_accessor(
                &mesh_object.vertex_indices,
                buffer_data,
                buffer_views,
                accessors,
            )?)
        } else {
            None
        };

        // Create mesh primitive attributes  
        let mut attributes = BTreeMap::new();
        attributes.insert(
            validation::Checked::Valid(mesh::Semantic::Positions),
            Index::new(position_accessor_index as u32),
        );

        if let Some(normal_idx) = normal_accessor_index {
            attributes.insert(
                validation::Checked::Valid(mesh::Semantic::Normals),
                Index::new(normal_idx as u32),
            );
        }

        if let Some(texcoord_idx) = texcoord_accessor_index {
            attributes.insert(
                validation::Checked::Valid(mesh::Semantic::TexCoords(0)),
                Index::new(texcoord_idx as u32),
            );
        }

        if let Some(joints_idx) = joints_accessor_index {
            attributes.insert(
                validation::Checked::Valid(mesh::Semantic::Joints(0)),
                Index::new(joints_idx as u32),
            );
        }

        if let Some(weights_idx) = weights_accessor_index {
            attributes.insert(
                validation::Checked::Valid(mesh::Semantic::Weights(0)),
                Index::new(weights_idx as u32),
            );
        }

        // Create primitive
        let primitive = mesh::Primitive {
            attributes,
            indices: indices_accessor_index.map(|i| Index::new(i as u32)),
            material: Some(Index::new(0)), // Use default material
            mode: validation::Checked::Valid(mesh::Mode::Triangles),
            targets: None,
            extensions: Default::default(),
            extras: Default::default(),
        };

        primitives.push(primitive);

        // Create mesh
        let gltf_mesh = Mesh {
            name: Some(mesh_object.name.clone()),
            primitives,
            weights: None,
            extensions: Default::default(),
            extras: Default::default(),
        };

        meshes.push(gltf_mesh);
    }

    Ok(meshes.len())
}

fn process_skeleton_data(
    skel_data: &SkelData,
    nodes: &mut Vec<Node>,
) -> Result<()> {
    // Create nodes for bones
    for (bone_index, bone) in skel_data.bones.iter().enumerate() {
        let transform_matrix = glam::Mat4::from_cols_array_2d(&bone.transform);
        let (scale, rotation, translation) = transform_matrix.to_scale_rotation_translation();

        let children: Vec<Index<Node>> = skel_data.bones
            .iter()
            .enumerate()
            .filter_map(|(child_index, child_bone)| {
                if child_bone.parent_index == Some(bone_index) {
                    Some(Index::new(child_index as u32))
                } else {
                    None
                }
            })
            .collect();

        let node = Node {
            name: Some(bone.name.clone()),
            translation: Some([translation.x, translation.y, translation.z]),
            rotation: Some(scene::UnitQuaternion([rotation.x, rotation.y, rotation.z, rotation.w])),
            scale: Some([scale.x, scale.y, scale.z]),
            children: if children.is_empty() { None } else { Some(children) },
            camera: None,
            mesh: None,
            skin: None,
            matrix: None,
            weights: None,
            extensions: Default::default(),
            extras: Default::default(),
        };

        nodes.push(node);
    }

    Ok(())
}

fn create_mesh_nodes(
    mesh_count: usize,
    _skeleton_node_offset: usize,
    skin_index: Option<usize>,
    nodes: &mut Vec<Node>,
) {
    // Create a node for each mesh
    for mesh_index in 0..mesh_count {
        let mesh_node = Node {
            name: Some(format!("MeshNode_{}", mesh_index)),
            translation: None,
            rotation: None,
            scale: None,
            children: None,
            camera: None,
            mesh: Some(Index::new(mesh_index as u32)),
            skin: skin_index.map(|i| Index::new(i as u32)),
            matrix: None,
            weights: None,
            extensions: Default::default(),
            extras: Default::default(),
        };
        
        nodes.push(mesh_node);
    }
}

fn create_vec3_accessor(
    data: &[[f32; 3]],
    buffer_data: &mut Vec<u8>,
    buffer_views: &mut Vec<BufferView>,
    accessors: &mut Vec<Accessor>,
    accessor_type: &str,
) -> Result<usize> {
    let byte_offset = buffer_data.len();
    let byte_length = data.len() * 3 * 4; // 3 components * 4 bytes per f32
    
    // Convert Vec3 data to bytes
    for vec in data {
        for component in vec {
            buffer_data.extend_from_slice(&component.to_le_bytes());
        }
    }

    // Create buffer view
    let buffer_view = BufferView {
        buffer: Index::new(0),
        byte_offset: Some(validation::USize64::from(byte_offset)),
        byte_length: validation::USize64::from(byte_length),
        byte_stride: Some(buffer::Stride(12)), // 3 * 4 bytes
        target: Some(validation::Checked::Valid(buffer::Target::ArrayBuffer)),
        name: None,
        extensions: Default::default(),
        extras: Default::default(),
    };
    buffer_views.push(buffer_view);
    let buffer_view_index = buffer_views.len() - 1;

    // Calculate min/max for position data
    let (min, max) = if accessor_type == "POSITION" {
        let mut min_vals = [f32::INFINITY; 3];
        let mut max_vals = [f32::NEG_INFINITY; 3];
        
        for vec in data {
            for (i, &val) in vec.iter().enumerate() {
                min_vals[i] = min_vals[i].min(val);
                max_vals[i] = max_vals[i].max(val);
            }
        }
        (Some(min_vals.to_vec()), Some(max_vals.to_vec()))
    } else {
        (None, None)
    };

    let accessor = Accessor {
        buffer_view: Some(Index::new(buffer_view_index as u32)),
        byte_offset: Some(validation::USize64::from(0u64)),
        component_type: validation::Checked::Valid(accessor::GenericComponentType(accessor::ComponentType::F32)),
        count: validation::USize64::from(data.len()),
        type_: validation::Checked::Valid(accessor::Type::Vec3),
        min: min.map(|v| serde_json::Value::Array(v.into_iter().map(serde_json::Value::from).collect())),
        max: max.map(|v| serde_json::Value::Array(v.into_iter().map(serde_json::Value::from).collect())),
        sparse: None,
        normalized: false,
        name: None,
        extensions: Default::default(),
        extras: Default::default(),
    };

    accessors.push(accessor);
    Ok(accessors.len() - 1)
}

fn create_vec2_accessor(
    data: &[[f32; 2]],
    buffer_data: &mut Vec<u8>,
    buffer_views: &mut Vec<BufferView>,
    accessors: &mut Vec<Accessor>,
    _accessor_type: &str,
) -> Result<usize> {
    let byte_offset = buffer_data.len();
    let byte_length = data.len() * 2 * 4; // 2 components * 4 bytes per f32
    
    // Convert Vec2 data to bytes
    for vec in data {
        for component in vec {
            buffer_data.extend_from_slice(&component.to_le_bytes());
        }
    }

    // Create buffer view
    let buffer_view = BufferView {
        buffer: Index::new(0),
        byte_offset: Some(validation::USize64::from(byte_offset)),
        byte_length: validation::USize64::from(byte_length),
        byte_stride: Some(buffer::Stride(8)), // 2 * 4 bytes
        target: Some(validation::Checked::Valid(buffer::Target::ArrayBuffer)),
        name: None,
        extensions: Default::default(),
        extras: Default::default(),
    };
    buffer_views.push(buffer_view);
    let buffer_view_index = buffer_views.len() - 1;

    let accessor = Accessor {
        buffer_view: Some(Index::new(buffer_view_index as u32)),
        byte_offset: Some(validation::USize64::from(0u64)),
        component_type: validation::Checked::Valid(accessor::GenericComponentType(accessor::ComponentType::F32)),
        count: validation::USize64::from(data.len()),
        type_: validation::Checked::Valid(accessor::Type::Vec2),
        min: None,
        max: None,
        sparse: None,
        normalized: false,
        name: None,
        extensions: Default::default(),
        extras: Default::default(),
    };

    accessors.push(accessor);
    Ok(accessors.len() - 1)
}

fn create_indices_accessor(
    indices: &[u32],
    buffer_data: &mut Vec<u8>,
    buffer_views: &mut Vec<BufferView>,
    accessors: &mut Vec<Accessor>,
) -> Result<usize> {
    let byte_offset = buffer_data.len();
    let byte_length = indices.len() * 4; // 4 bytes per u32
    
    // Convert indices to bytes
    for &index in indices {
        buffer_data.extend_from_slice(&index.to_le_bytes());
    }

    // Create buffer view
    let buffer_view = BufferView {
        buffer: Index::new(0),
        byte_offset: Some(validation::USize64::from(byte_offset)),
        byte_length: validation::USize64::from(byte_length),
        byte_stride: None,
        target: Some(validation::Checked::Valid(buffer::Target::ElementArrayBuffer)),
        name: None,
        extensions: Default::default(),
        extras: Default::default(),
    };
    buffer_views.push(buffer_view);
    let buffer_view_index = buffer_views.len() - 1;

    let accessor = Accessor {
        buffer_view: Some(Index::new(buffer_view_index as u32)),
        byte_offset: Some(validation::USize64::from(0u64)),
        component_type: validation::Checked::Valid(accessor::GenericComponentType(accessor::ComponentType::U32)),
        count: validation::USize64::from(indices.len()),
        type_: validation::Checked::Valid(accessor::Type::Scalar),
        min: None,
        max: None,
        sparse: None,
        normalized: false,
        name: None,
        extensions: Default::default(),
        extras: Default::default(),
    };

    accessors.push(accessor);
    Ok(accessors.len() - 1)
}

fn convert_vector_data_to_vec3(data: &ssbh_data::mesh_data::VectorData) -> Result<Vec<[f32; 3]>> {
    use ssbh_data::mesh_data::VectorData;
    
    match data {
        VectorData::Vector3(vec3_data) => Ok(vec3_data.clone()),
        VectorData::Vector4(vec4_data) => {
            Ok(vec4_data.iter().map(|v| [v[0], v[1], v[2]]).collect())
        }
        VectorData::Vector2(vec2_data) => {
            Ok(vec2_data.iter().map(|v| [v[0], v[1], 0.0]).collect())
        }
    }
}

fn convert_vector_data_to_vec2(data: &ssbh_data::mesh_data::VectorData) -> Result<Vec<[f32; 2]>> {
    use ssbh_data::mesh_data::VectorData;
    
    match data {
        VectorData::Vector2(vec2_data) => Ok(vec2_data.clone()),
        VectorData::Vector3(vec3_data) => {
            Ok(vec3_data.iter().map(|v| [v[0], v[1]]).collect())
        }
        VectorData::Vector4(vec4_data) => {
            Ok(vec4_data.iter().map(|v| [v[0], v[1]]).collect())
        }
    }
}

fn create_joints_accessor(
    joints_data: &[[u16; 4]],
    buffer_data: &mut Vec<u8>,
    buffer_views: &mut Vec<BufferView>,
    accessors: &mut Vec<Accessor>,
) -> Result<usize> {
    let byte_offset = buffer_data.len();
    let byte_length = joints_data.len() * 4 * 2; // 4 components * 2 bytes per u16
    
    // Convert joints data to bytes
    for joints in joints_data {
        for &joint in joints {
            buffer_data.extend_from_slice(&joint.to_le_bytes());
        }
    }

    // Create buffer view
    let buffer_view = BufferView {
        buffer: Index::new(0),
        byte_offset: Some(validation::USize64::from(byte_offset)),
        byte_length: validation::USize64::from(byte_length),
        byte_stride: Some(buffer::Stride(8)), // 4 * 2 bytes
        target: Some(validation::Checked::Valid(buffer::Target::ArrayBuffer)),
        name: None,
        extensions: Default::default(),
        extras: Default::default(),
    };
    buffer_views.push(buffer_view);
    let buffer_view_index = buffer_views.len() - 1;

    let accessor = Accessor {
        buffer_view: Some(Index::new(buffer_view_index as u32)),
        byte_offset: Some(validation::USize64::from(0u64)),
        component_type: validation::Checked::Valid(accessor::GenericComponentType(accessor::ComponentType::U16)),
        count: validation::USize64::from(joints_data.len()),
        type_: validation::Checked::Valid(accessor::Type::Vec4),
        min: None,
        max: None,
        sparse: None,
        normalized: false,
        name: None,
        extensions: Default::default(),
        extras: Default::default(),
    };

    accessors.push(accessor);
    Ok(accessors.len() - 1)
}

fn create_weights_accessor(
    weights_data: &[[f32; 4]],
    buffer_data: &mut Vec<u8>,
    buffer_views: &mut Vec<BufferView>,
    accessors: &mut Vec<Accessor>,
) -> Result<usize> {
    let byte_offset = buffer_data.len();
    let byte_length = weights_data.len() * 4 * 4; // 4 components * 4 bytes per f32
    
    // Convert weights data to bytes
    for weights in weights_data {
        for &weight in weights {
            buffer_data.extend_from_slice(&weight.to_le_bytes());
        }
    }

    // Create buffer view
    let buffer_view = BufferView {
        buffer: Index::new(0),
        byte_offset: Some(validation::USize64::from(byte_offset)),
        byte_length: validation::USize64::from(byte_length),
        byte_stride: Some(buffer::Stride(16)), // 4 * 4 bytes
        target: Some(validation::Checked::Valid(buffer::Target::ArrayBuffer)),
        name: None,
        extensions: Default::default(),
        extras: Default::default(),
    };
    buffer_views.push(buffer_view);
    let buffer_view_index = buffer_views.len() - 1;

    let accessor = Accessor {
        buffer_view: Some(Index::new(buffer_view_index as u32)),
        byte_offset: Some(validation::USize64::from(0u64)),
        component_type: validation::Checked::Valid(accessor::GenericComponentType(accessor::ComponentType::F32)),
        count: validation::USize64::from(weights_data.len()),
        type_: validation::Checked::Valid(accessor::Type::Vec4),
        min: None,
        max: None,
        sparse: None,
        normalized: false,
        name: None,
        extensions: Default::default(),
        extras: Default::default(),
    };

    accessors.push(accessor);
    Ok(accessors.len() - 1)
}

fn create_skin(
    skel_data: &SkelData,
    buffer_data: &mut Vec<u8>,
    buffer_views: &mut Vec<BufferView>,
    accessors: &mut Vec<Accessor>,
    skins: &mut Vec<Skin>,
) -> Result<usize> {
    // Calculate world transforms for each bone in bind pose
    let mut world_transforms = vec![glam::Mat4::IDENTITY; skel_data.bones.len()];
    let mut calculated = vec![false; skel_data.bones.len()];
    
    // Function to recursively calculate world transform for a bone
    fn calculate_world_transform(
        bone_index: usize,
        skel_data: &SkelData,
        world_transforms: &mut [glam::Mat4],
        calculated: &mut [bool],
    ) {
        if calculated[bone_index] {
            return;
        }
        
        let bone = &skel_data.bones[bone_index];
        let local_transform = glam::Mat4::from_cols_array_2d(&bone.transform);
        
        if let Some(parent_index) = bone.parent_index {
            // Ensure parent is calculated first
            calculate_world_transform(parent_index, skel_data, world_transforms, calculated);
            // Child bone: world_transform = parent_world_transform * local_transform
            world_transforms[bone_index] = world_transforms[parent_index] * local_transform;
        } else {
            // Root bone: world_transform = local_transform
            world_transforms[bone_index] = local_transform;
        }
        
        calculated[bone_index] = true;
    }
    
    // Calculate world transforms for all bones
    for bone_index in 0..skel_data.bones.len() {
        calculate_world_transform(bone_index, skel_data, &mut world_transforms, &mut calculated);
    }
    
    // Create inverse bind matrices from world transforms
    let mut inverse_bind_matrices = Vec::new();
    
    for world_transform in &world_transforms {
        // For GLTF, we need the inverse of the world transform at bind time
        let inverse_bind_matrix = world_transform.inverse();
        let matrix_array = inverse_bind_matrix.to_cols_array();
        inverse_bind_matrices.push(matrix_array);
    }
    
    // Create accessor for inverse bind matrices
    let inverse_bind_matrices_accessor = create_mat4_accessor(
        &inverse_bind_matrices,
        buffer_data,
        buffer_views,
        accessors,
    )?;

    // Create joint indices (0..bone_count)
    let joints: Vec<Index<Node>> = (0..skel_data.bones.len())
        .map(|i| Index::new(i as u32))
        .collect();

    let skin = Skin {
        inverse_bind_matrices: Some(Index::new(inverse_bind_matrices_accessor as u32)),
        skeleton: None, // Could set to root bone if needed
        joints,
        name: Some("Skeleton".to_string()),
        extensions: Default::default(),
        extras: Default::default(),
    };

    skins.push(skin);
    Ok(skins.len() - 1)
}

fn create_mat4_accessor(
    matrices: &[[f32; 16]],
    buffer_data: &mut Vec<u8>,
    buffer_views: &mut Vec<BufferView>,
    accessors: &mut Vec<Accessor>,
) -> Result<usize> {
    let byte_offset = buffer_data.len();
    let byte_length = matrices.len() * 16 * 4; // 16 components * 4 bytes per f32

    // Convert matrices to bytes
    for matrix in matrices {
        for &component in matrix {
            buffer_data.extend_from_slice(&component.to_le_bytes());
        }
    }

    // Create buffer view
    let buffer_view = BufferView {
        buffer: Index::new(0),
        byte_offset: Some(validation::USize64::from(byte_offset)),
        byte_length: validation::USize64::from(byte_length),
        byte_stride: Some(buffer::Stride(64)), // 16 * 4 bytes
        target: Some(validation::Checked::Valid(buffer::Target::ArrayBuffer)),
        name: None,
        extensions: Default::default(),
        extras: Default::default(),
    };
    buffer_views.push(buffer_view);
    let buffer_view_index = buffer_views.len() - 1;

    let accessor = Accessor {
        buffer_view: Some(Index::new(buffer_view_index as u32)),
        byte_offset: Some(validation::USize64::from(0u64)),
        component_type: validation::Checked::Valid(accessor::GenericComponentType(accessor::ComponentType::F32)),
        count: validation::USize64::from(matrices.len()),
        type_: validation::Checked::Valid(accessor::Type::Mat4),
        min: None,
        max: None,
        sparse: None,
        normalized: false,
        name: None,
        extensions: Default::default(),
        extras: Default::default(),
    };

    accessors.push(accessor);
    Ok(accessors.len() - 1)
}

fn load_model_from_json_files() -> Result<ModelFolder> {
    // Load skeleton data from skeleton.json
    let skeleton_path = Path::new("skeleton.json");
    let skel_data = if skeleton_path.exists() {
        let skeleton_json = fs::read_to_string(skeleton_path)?;
        let skel: SkelData = serde_json::from_str(&skeleton_json)?;
        Some(skel)
    } else {
        None
    };

    // Load mesh data from model.json
    let model_path = Path::new("model.json");
    let mesh_data = if model_path.exists() {
        let model_json = fs::read_to_string(model_path)?;
        let mesh: MeshData = serde_json::from_str(&model_json)?;
        Some(mesh)
    } else {
        None
    };

    // Construct ModelFolder with loaded data
    let mut model_folder = ModelFolder {
        meshes: Vec::new(),
        skels: Vec::new(),
        matls: Vec::new(),
        modls: Vec::new(),
        adjs: Vec::new(),
        anims: Vec::new(),
        hlpbs: Vec::new(),
        nutexbs: Vec::new(),
        meshexes: Vec::new(),
        xmbs: Vec::new(),
    };

    // Add loaded data to model folder
    if let Some(mesh) = mesh_data {
        model_folder.meshes.push(("model.numsheb".to_string(), Some(mesh)));
    }

    if let Some(skel) = skel_data {
        model_folder.skels.push(("model.nusktb".to_string(), Some(skel)));
    }

    Ok(model_folder)
}
