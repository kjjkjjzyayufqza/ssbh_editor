use anyhow::Result;
use ssbh_wgpu::ModelFolder;

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
        let response = egui::Window::new("Export NUMDLB Scene")
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
                
                let mut export_clicked = false;
                let mut cancel_clicked = false;
                ui.horizontal(|ui| {
                    let can_export = !state.config.output_directory.as_os_str().is_empty() 
                        && !state.config.base_filename.is_empty()
                        && (state.config.export_mesh || state.config.export_skeleton || state.config.export_modl);
                    
                    if ui.add_enabled(can_export, egui::Button::new("Export")).clicked() {
                        export_clicked = true;
                    }
                    
                    if ui.button("Cancel").clicked() {
                        cancel_clicked = true;
                    }
                });
                
                (export_clicked, cancel_clicked)
            });
            
        if let Some(inner_response) = response {
            if let Some((export_clicked, cancel_clicked)) = inner_response.inner {
                if export_clicked {
                    result = Some(state.config.clone());
                    state.is_open = false;
                }
                if cancel_clicked {
                    state.is_open = false;
                }
            }
        }
    }
    
    result
}
