use crate::error::{Error, Result};
use crate::kicad_models::*;
use regex::Regex;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf; // We need to add `regex` to our dependencies in Cargo.toml

const KICAD_SYM_HEADER: &str = r#"(kicad_symbol_lib (version 20211014) (generator easyeda2kicad_rs)
"#;

const KICAD_SYM_FOOTER: &str = r#")
"#;

/// Manages the output library structure.
pub struct KicadLibrary {
    pub path: PathBuf,
}

impl KicadLibrary {
    /// Creates the necessary directories for symbols, footprints, and 3D models.
    pub fn setup_directories(&self) -> Result<()> {
        fs::create_dir_all(self.path.join("footprints.pretty"))?;
        fs::create_dir_all(self.path.join("symbols"))?;
        fs::create_dir_all(self.path.join("3dmodels.3dshapes"))?;
        Ok(())
    }

    /// Adds a symbol to the symbol library file.
    pub fn add_symbol(&self, symbol: &KiSymbol) -> Result<()> {
        let lib_path = self.path.join("symbols/lib.kicad_sym");
        let symbol_content = symbol.to_kicad_lib_entry();

        // --- Check for Duplicates ---
        if lib_path.exists() {
            let mut file_content = String::new();
            File::open(&lib_path)?.read_to_string(&mut file_content)?;

            // Regex to find (symbol "SYMBOL_NAME" ... )
            // We escape the name to handle special characters.
            let pattern = format!(r#"\(\s*symbol\s*"{}"\s*.*\)"#, regex::escape(&symbol.name));
            let re = Regex::new(&pattern).map_err(|e| Error::ParseError(e.to_string()))?;

            if re.is_match(&file_content) {
                println!(
                    "Symbol '{}' already exists in the library. Skipping.",
                    symbol.name
                );
                // Optionally, here you could implement logic to UPDATE the symbol instead of skipping.
                return Ok(());
            }
        }

        // --- Open or Create the File ---
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lib_path)?;

        let metadata = file.metadata()?;

        if metadata.len() == 0 {
            // File is new or empty, write header, symbol, and footer
            file.write_all(KICAD_SYM_HEADER.as_bytes())?;
            file.write_all(symbol_content.as_bytes())?;
            file.write_all(KICAD_SYM_FOOTER.as_bytes())?;
            println!("Created new symbol library and added '{}'.", symbol.name);
        } else {
            // File exists, amend it
            // Seek to the end of the file, but before the last character (')')
            file.seek(SeekFrom::End(-(KICAD_SYM_FOOTER.len() as i64)))?;

            // Write the new symbol content, followed by the footer again.
            // This effectively inserts the symbol before the final ')'
            file.write_all(symbol_content.as_bytes())?;
            file.write_all(KICAD_SYM_FOOTER.as_bytes())?;
            println!("Appended symbol '{}' to the existing library.", symbol.name);
        }

        Ok(())
    }

    /// Writes a footprint to its own .kicad_mod file.
    pub fn add_footprint(&self, footprint: &KiFootprint) -> Result<()> {
        let fp_path = self
            .path
            .join(format!("footprints.pretty/{}.kicad_mod", footprint.name));
        let content = footprint.to_kicad_mod_entry();
        fs::write(fp_path, content)?;
        Ok(())
    }

    /// Writes the 3D model files (.wrl, .step).
    pub fn add_3d_model(&self, model: &Ki3dModel) -> Result<()> {
        let base_path = self.path.join("3dmodels.3dshapes").join(&model.name);
        if let Some(wrl_data) = &model.wrl_data {
            fs::write(base_path.with_extension("wrl"), wrl_data)?;
        }
        if let Some(step_data) = &model.step_data {
            fs::write(base_path.with_extension("step"), step_data)?;
        }
        Ok(())
    }
}
