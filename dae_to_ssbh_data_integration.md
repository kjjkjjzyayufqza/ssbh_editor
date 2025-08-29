# DAE to ssbh_data Integration Design

## Current Issue
Current `convert_dae_to_ssbh_files()` manually constructs `MeshData`, `ModlData`, `MatlData` objects. This bypasses ssbh_data's built-in validation and standardization.

## Correct Approach
Use ssbh_data's factory methods and validation pipeline instead of manual object construction.

## Code Changes Required

### 1. Replace Manual MeshData Construction

**Current (Incorrect):**
```rust
let mesh_object = MeshObjectData {
    name: dae_mesh.name.clone(),
    subindex: index as u64,
    positions: vec![AttributeData {
        name: String::new(),
        data: VectorData::Vector3(vertices),
    }],
    // ... manual field assignment
};

Ok(MeshData {
    major_version: 1,
    minor_version: 10,
    objects: mesh_objects,
})
```

**Correct (Use ssbh_data):**
```rust
use ssbh_data::mesh_data::MeshData;

let mut mesh_data = MeshData::new();
for (index, dae_mesh) in meshes.iter().enumerate() {
    let vertices = apply_transforms(&dae_mesh.vertices, config);
    let normals = apply_normal_transforms(&dae_mesh.normals, config);
    
    mesh_data.add_mesh_object(
        &dae_mesh.name,
        index as u64,
        &vertices,
        &normals,
        &dae_mesh.uvs,
        &dae_mesh.indices,
        &convert_bone_influences(&dae_mesh.bone_influences)
    )?;
}
```

### 2. Replace Manual ModlData Construction

**Current (Incorrect):**
```rust
let entry = ModlEntryData {
    mesh_object_name: mesh.name.clone(),
    mesh_object_subindex: mesh_index as u64,
    material_label,
};

Ok(ModlData {
    major_version: 1,
    minor_version: 0,
    model_name: config.base_filename.clone(),
    // ... manual field assignment
})
```

**Correct (Use ssbh_data):**
```rust
use ssbh_data::modl_data::ModlData;

let mut modl_data = ModlData::new(&config.base_filename);
modl_data.set_mesh_file(&format!("{}.numshb", config.base_filename));
modl_data.add_material_file(&format!("{}.numatb", config.base_filename));

for (mesh_index, mesh) in meshes.iter().enumerate() {
    let material_label = resolve_material_label(mesh, materials);
    modl_data.add_entry(&mesh.name, mesh_index as u64, &material_label)?;
}
```

### 3. Replace Manual MatlData Construction

**Current (Incorrect):**
```rust
let default_material = MatlEntryData {
    material_label: "DefaultMaterial".to_string(),
    shader_label: "SFX_PBS_010002000800824f_opaque".to_string(),
    // ... manual field assignment
};

Ok(MatlData {
    major_version: 1,
    minor_version: 6,
    entries,
})
```

**Correct (Use ssbh_data):**
```rust
use ssbh_data::matl_data::MatlData;

let mut matl_data = MatlData::new();
matl_data.add_default_material("DefaultMaterial")?;

for dae_material in materials {
    matl_data.add_material_from_properties(
        &dae_material.name,
        &dae_material.diffuse_color,
        &dae_material.specular_color,
        &dae_material.emission_color,
        &dae_material.texture_paths
    )?;
}
```

### 4. Updated Function Signatures

```rust
fn convert_meshes_to_ssbh(meshes: &[DaeMesh], config: &DaeConvertConfig) -> Result<MeshData> {
    let mut mesh_data = MeshData::new();
    
    for (index, dae_mesh) in meshes.iter().enumerate() {
        if dae_mesh.vertices.is_empty() {
            continue;
        }
        
        let vertices = apply_transforms(&dae_mesh.vertices, config);
        let normals = apply_normal_transforms(&dae_mesh.normals, config);
        let bone_influences = convert_dae_bone_influences_to_ssbh(&dae_mesh.bone_influences);
        
        mesh_data.add_mesh_object(
            &dae_mesh.name,
            index as u64,
            &vertices,
            &normals,
            &dae_mesh.uvs,
            &dae_mesh.indices,
            &bone_influences
        )?;
    }
    
    Ok(mesh_data)
}

fn convert_model_to_ssbh(meshes: &[DaeMesh], materials: &[DaeMaterial], config: &DaeConvertConfig) -> Result<ModlData> {
    let mut modl_data = ModlData::new(&config.base_filename);
    modl_data.set_mesh_file(&format!("{}.numshb", config.base_filename));
    modl_data.add_material_file(&format!("{}.numatb", config.base_filename));
    
    for (mesh_index, mesh) in meshes.iter().enumerate() {
        let material_label = resolve_material_label(mesh, materials);
        modl_data.add_entry(&mesh.name, mesh_index as u64, &material_label)?;
    }
    
    Ok(modl_data)
}

fn convert_materials_to_ssbh(materials: &[DaeMaterial], _config: &DaeConvertConfig) -> Result<MatlData> {
    let mut matl_data = MatlData::new();
    matl_data.add_default_material("DefaultMaterial")?;
    
    for dae_material in materials {
        matl_data.add_material_from_properties(
            &dae_material.name,
            &dae_material.diffuse_color,
            &dae_material.specular_color,
            &dae_material.emission_color,
            &dae_material.texture_paths
        )?;
    }
    
    Ok(matl_data)
}
```

### 5. Validation Integration

```rust
pub fn convert_dae_to_ssbh_files(
    dae_scene: &DaeScene,
    config: &DaeConvertConfig,
) -> Result<ConvertedFiles> {
    let mut converted_files = ConvertedFiles::default();
    
    // Use ssbh_data factory methods instead of manual construction
    let mesh_data = convert_meshes_to_ssbh(&dae_scene.meshes, config)?;
    mesh_data.validate()?; // Use ssbh_data validation
    
    let modl_data = convert_model_to_ssbh(&dae_scene.meshes, &dae_scene.materials, config)?;
    modl_data.validate()?; // Use ssbh_data validation
    
    let matl_data = convert_materials_to_ssbh(&dae_scene.materials, config)?;
    matl_data.validate()?; // Use ssbh_data validation
    
    // Write using ssbh_data methods
    let mesh_path = config.output_directory.join(format!("{}.numshb", config.base_filename));
    mesh_data.write_to_file(&mesh_path)?;
    converted_files.numshb_path = Some(mesh_path);
    
    let modl_path = config.output_directory.join(format!("{}.numdlb", config.base_filename));
    modl_data.write_to_file(&modl_path)?;
    converted_files.numdlb_path = Some(modl_path);
    
    let matl_path = config.output_directory.join(format!("{}.numatb", config.base_filename));
    matl_data.write_to_file(&matl_path)?;
    converted_files.numatb_path = Some(matl_path);
    
    Ok(converted_files)
}
```

### 6. Helper Function for Material Resolution

```rust
fn resolve_material_label(mesh: &DaeMesh, materials: &[DaeMaterial]) -> String {
    if let Some(ref mat_name) = mesh.material_name {
        if materials.iter().any(|m| &m.name == mat_name) {
            mat_name.clone()
        } else {
            "DefaultMaterial".to_string()
        }
    } else {
        "DefaultMaterial".to_string()
    }
}
```

### 7. Remove Manual Version and Field Assignment

Delete all hardcoded version numbers and field assignments. Let ssbh_data handle:
- Version compatibility
- Default field values  
- Internal data structure consistency
- Format-specific validation rules
