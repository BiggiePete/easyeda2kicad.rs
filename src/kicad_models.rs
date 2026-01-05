// src/kicad_models.rs

use glam::Vec3; // Using glam for 3D vector math
use std::fmt::Write;

// --- 3D Model Structs ---

/// Represents a 3D model in KiCad format.
///
/// Contains both VRML and STEP format data along with placement information.
#[derive(Debug, Clone)]
pub struct Ki3dModel {
    pub name: String,
    pub wrl_data: Option<String>,
    pub step_data: Option<bytes::Bytes>,
    // KiCad placement info
    pub offset: Vec3,
    pub scale: Vec3,
    pub rotate: Vec3,
}

// --- Footprint Structs ---

#[derive(Debug)]
pub enum FpShape {
    Circle,
    Rect,
    Oval,
}

#[derive(Debug)]
pub struct FpPad {
    pub number: String,
    pub pad_type: String, // "smd" or "thru_hole"
    pub shape: FpShape,
    pub pos: (f32, f32),
    pub size: (f32, f32),
    pub layers: Vec<String>,
    pub rotation: f32,                  // in degrees
    pub drill: Option<f32>,             // drill diameter in mm for circular holes
    pub drill_oval: Option<(f32, f32)>, // (width, height) for oval/slot holes
}

#[derive(Debug)]
pub struct FpText {
    pub text_type: String, // "reference", "value"
    pub text: String,
    pub pos: (f32, f32),
    pub layer: String,
}

#[derive(Debug)]
pub struct KiFootprint {
    pub name: String,
    pub pads: Vec<FpPad>,
    pub texts: Vec<FpText>,
    pub model_3d: Option<Ki3dModel>,
}

#[derive(Debug)]
pub enum KiPinType {
    Input,
    Output,
    Bidirectional,
    PowerIn,
    Passive,
    Unspecified,
}

#[derive(Debug)]
pub struct KiSymbolPin {
    pub name: String,
    pub number: String,
    pub pin_type: KiPinType,
    pub length: f32,
    pub pos: (f32, f32),
    pub rotation: i32,
}

#[derive(Debug)]
pub struct KiSymbolRect {
    pub start: (f32, f32),
    pub end: (f32, f32),
}

#[derive(Debug)]
pub struct KiSymbol {
    pub name: String,
    pub reference: String,
    pub footprint: String,
    pub datasheet: String,
    pub lcsc_part: Option<String>,
    pub is_extended: bool,
    pub pins: Vec<KiSymbolPin>,
    pub rectangles: Vec<KiSymbolRect>,
}

impl KiSymbol {
    pub fn to_kicad_lib_entry(&self) -> String {
        let mut out = String::new();
        writeln!(
            &mut out,
            "(symbol \"{}\" (in_bom yes) (on_board yes)",
            self.name
        )
        .unwrap();

        // --- Properties ---
        writeln!(&mut out, "  (property \"Reference\" \"{}\" (id 0) (at 0 2.54 0) (effects (font (size 1.27 1.27))))", self.reference).unwrap();
        writeln!(
            &mut out,
            "  (property \"Value\" \"{}\" (id 1) (at 0 -2.54 0) (effects (font (size 1.27 1.27))))",
            self.name
        )
        .unwrap();
        writeln!(&mut out, "  (property \"Footprint\" \"{}\" (id 2) (at 0 0 0) (effects (font (size 1.27 1.27)) hide))", self.footprint).unwrap();
        writeln!(&mut out, "  (property \"Datasheet\" \"{}\" (id 3) (at 0 0 0) (effects (font (size 1.27 1.27)) hide))", self.datasheet).unwrap();
        if let Some(lcsc) = &self.lcsc_part {
            writeln!(&mut out, "  (property \"LCSC Part\" \"{}\" (id 4) (at 0 0 0) (effects (font (size 1.27 1.27)) hide))", lcsc).unwrap();
        }
        writeln!(&mut out, "  (property \"Extended\" \"{}\" (id 5) (at 0 0 0) (effects (font (size 1.27 1.27)) hide))", self.is_extended).unwrap();

        // --- Symbol Graphics ---
        writeln!(&mut out, "  (symbol \"{}_1_1\"", self.name).unwrap();

        for rect in &self.rectangles {
            writeln!(&mut out, "    (rectangle (start {} {}) (end {} {}) (stroke (width 0.254) (type default) (color 0 0 0 0)) (fill (type background)))",
                rect.start.0, rect.start.1, rect.end.0, rect.end.1).unwrap();
        }

        for pin in &self.pins {
            let pin_type_str = match pin.pin_type {
                KiPinType::Input => "input",
                KiPinType::Output => "output",
                KiPinType::Bidirectional => "bidirectional",
                KiPinType::PowerIn => "power_in",
                KiPinType::Passive => "passive",
                KiPinType::Unspecified => "unspecified",
            };

            let pin_name = if pin.name.starts_with('~') {
                format!("\"~{{{}}}\"", &pin.name[1..])
            } else {
                format!("\"{}\"", pin.name)
            };

            writeln!(
                &mut out,
                r#"    (pin {} line (at {} {} {}) (length {})
      (name {} (effects (font (size 1.27 1.27))))
      (number "{}" (effects (font (size 1.27 1.27))))
    )"#,
                pin_type_str, pin.pos.0, pin.pos.1, pin.rotation, pin.length, pin_name, pin.number
            )
            .unwrap();
        }

        writeln!(&mut out, "  )\n)").unwrap(); // Close symbol "{name}_1" and main symbol
        out
    }
}

impl KiFootprint {
    /// Generates the full S-expression string for a .kicad_mod file.
    pub fn to_kicad_mod_entry(&self) -> String {
        let mut out = String::new();
        writeln!(&mut out, "(module {} (layer F.Cu)", self.name).unwrap();

        // Add texts (reference, value, etc.)
        for text in &self.texts {
            writeln!(
                &mut out,
                "  (fp_text {} {} (at {} {}) (layer {}) (effects (font (size 1 1) (thickness 0.15))))",
                text.text_type, text.text, text.pos.0, text.pos.1, text.layer
            ).unwrap();
        }

        // Add 3D model path
        if let Some(model) = &self.model_3d {
            writeln!(
                &mut out,
                r#"  (model "../3dmodels.3dshapes/{}.wrl"
    (offset (xyz {} {} {}))
    (scale (xyz {} {} {}))
    (rotate (xyz {} {} {}))
  )"#,
                model.name,
                model.offset.x,
                model.offset.y,
                model.offset.z,
                model.scale.x,
                model.scale.y,
                model.scale.z,
                model.rotate.x,
                model.rotate.y,
                model.rotate.z
            )
            .unwrap();
        }

        // Add pads
        for pad in &self.pads {
            let shape_str = match pad.shape {
                FpShape::Circle => "circle",
                FpShape::Rect => "rect",
                FpShape::Oval => "oval",
            };
            let layers_str = pad.layers.join(" ");

            if let Some((width, height)) = pad.drill_oval {
                // Oval/slot hole
                writeln!(
                    &mut out,
                    "  (pad {} {} {} (at {} {} {}) (size {} {}) (layers {}) (drill oval {} {}))",
                    pad.number,
                    pad.pad_type,
                    shape_str,
                    pad.pos.0,
                    pad.pos.1,
                    pad.rotation,
                    pad.size.0,
                    pad.size.1,
                    layers_str,
                    width,
                    height
                )
                .unwrap();
            } else if let Some(drill_dia) = pad.drill {
                // Circular hole
                writeln!(
                    &mut out,
                    "  (pad {} {} {} (at {} {} {}) (size {} {}) (layers {}) (drill {}))",
                    pad.number,
                    pad.pad_type,
                    shape_str,
                    pad.pos.0,
                    pad.pos.1,
                    pad.rotation,
                    pad.size.0,
                    pad.size.1,
                    layers_str,
                    drill_dia
                )
                .unwrap();
            } else {
                // SMD pad (no drill)
                writeln!(
                    &mut out,
                    "  (pad {} {} {} (at {} {} {}) (size {} {}) (layers {}))",
                    pad.number,
                    pad.pad_type,
                    shape_str,
                    pad.pos.0,
                    pad.pos.1,
                    pad.rotation,
                    pad.size.0,
                    pad.size.1,
                    layers_str
                )
                .unwrap();
            }
        }

        writeln!(&mut out, ")").unwrap();
        out
    }
}
