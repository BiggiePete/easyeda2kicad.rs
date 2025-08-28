// src/lib.rs

pub mod api;
pub mod converter;
pub mod easyeda_models;
pub mod error;
pub mod file_writer;
pub mod importer;
pub mod kicad_models;

use crate::error::Result;
use std::path::Path;

pub async fn import_component(lcsc_id: &str, output_dir: &Path) -> Result<()> {
    println!("Fetching data for LCSC ID: {}", lcsc_id);
    let api = api::EasyedaApi::new();
    let kicad_lib = file_writer::KicadLibrary {
        path: output_dir.to_path_buf(),
    };
    kicad_lib.setup_directories()?;

    let cad_data = api.get_cad_data_of_component(lcsc_id).await?;

    // --- SYMBOL ---
    let ee_symbol = importer::import_symbol(&cad_data)?;
    let ki_symbol = converter::convert_symbol(ee_symbol)?;
    let ee_footprint = importer::import_footprint(&cad_data)?;

    kicad_lib.add_symbol(&ki_symbol)?;
    println!("Successfully generated symbol: {}", ki_symbol.name);

    // --- 3D MODEL ---
    let ki_model = if let Some(mut ee_model_info) = importer::import_3d_model_info(&cad_data)? {
        println!("Found 3D model: {}", ee_model_info.name);
        let (raw_obj, step) = tokio::join!(
            api.get_raw_3d_model_obj(&ee_model_info.uuid),
            api.get_step_3d_model(&ee_model_info.uuid)
        );
        ee_model_info.raw_obj = raw_obj.ok();
        ee_model_info.step = step.ok();
        let model = converter::convert_3d_model(ee_model_info)?;
        kicad_lib.add_3d_model(&model)?;
        println!("Successfully generated 3D model: {}", model.name);
        Some(model)
    } else {
        println!("No 3D model found for this component.");
        None
    };

    // --- FOOTPRINT ---
    // Pass the 3D model data to the footprint converter
    let ki_footprint = converter::convert_footprint(ee_footprint, ki_model)?;
    kicad_lib.add_footprint(&ki_footprint)?;
    println!("Successfully generated footprint: {}", ki_footprint.name);

    println!("\nImport complete. Files are located in: {:?}", output_dir);
    Ok(())
}
