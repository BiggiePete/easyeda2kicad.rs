// src/converter.rs

use crate::{easyeda_models::*, error::Result, kicad_models::*};
use glam::Vec3;

/// Converts an EasyEDA symbol to a KiCad symbol. (Still a stub)
pub fn convert_symbol(ee_symbol: EeSymbol) -> Result<KiSymbol> {
    let (bbox_x, bbox_y) = ee_symbol.bbox;
    let mut ki_pins = Vec::new();

    for ee_pin in ee_symbol.pins {
        ki_pins.push(KiSymbolPin {
            name: ee_pin.name,
            number: ee_pin.number,
            pin_type: map_pin_type(&ee_pin.pin_type),
            length: ee_to_mm(ee_pin.pin_length),
            pos: (
                ee_to_mm(ee_pin.pos_x - bbox_x),
                ee_to_mm(-(ee_pin.pos_y - bbox_y)),
            ),
            rotation: (ee_pin.rotation + 180) % 360, // KiCad rotation is often different
        });
    }

    let mut ki_rects = Vec::new();
    for ee_rect in ee_symbol.rectangles {
        let start_x = ee_to_mm(ee_rect.x - bbox_x);
        let start_y = ee_to_mm(-(ee_rect.y - bbox_y));
        let end_x = start_x + ee_to_mm(ee_rect.width);
        let end_y = start_y - ee_to_mm(ee_rect.height);
        ki_rects.push(KiSymbolRect {
            start: (start_x, start_y),
            end: (end_x, end_y),
        });
    }

    Ok(KiSymbol {
        name: ee_symbol.info.name,
        reference: ee_symbol.info.prefix,
        footprint: ee_symbol.info.package.unwrap_or_default(),
        datasheet: ee_symbol.info.datasheet.unwrap_or_default(),
        pins: ki_pins,
        rectangles: ki_rects,
    })
}

/// Converts an EasyEDA footprint to a KiCad footprint. (Now implemented)
fn ee_to_mm(val: f32) -> f32 {
    val * 0.254
}

/// Maps EasyEDA layer IDs to KiCad layer names.
fn map_layer(layer_id: i32, is_smd: bool) -> Vec<String> {
    match layer_id {
        1 => {
            // Top Layer
            if is_smd {
                vec![
                    "F.Cu".to_string(),
                    "F.Paste".to_string(),
                    "F.Mask".to_string(),
                ]
            } else {
                vec!["*.Cu".to_string(), "*.Mask".to_string()] // Through-hole
            }
        }
        2 => vec![
            "B.Cu".to_string(),
            "B.Paste".to_string(),
            "B.Mask".to_string(),
        ], // Bottom Layer
        3 => vec!["F.SilkS".to_string()], // Top Silkscreen
        4 => vec!["B.SilkS".to_string()],
        13 => vec!["F.Fab".to_string()],
        15 => vec!["Dwgs.User".to_string()],
        _ => vec!["F.Fab".to_string()], // Default fallback
    }
}

/// Maps EasyEDA pad shapes to KiCad pad shapes.
fn map_shape(shape: &str) -> FpShape {
    match shape {
        "ELLIPSE" => FpShape::Circle,
        "RECT" => FpShape::Rect,
        "OVAL" => FpShape::Oval,
        _ => FpShape::Rect, // Default fallback
    }
}
fn map_pin_type(ee_type: &str) -> KiPinType {
    match ee_type {
        "1" => KiPinType::Input,
        "2" => KiPinType::Output,
        "3" => KiPinType::Bidirectional,
        "4" => KiPinType::PowerIn,
        _ => KiPinType::Passive,
    }
}
/// Converts an EasyEDA footprint to a KiCad footprint.
pub fn convert_footprint(
    ee_footprint: EeFootprint,
    ki_model: Option<Ki3dModel>,
) -> Result<KiFootprint> {
    let mut ki_pads = Vec::new();
    let (bbox_x, bbox_y) = ee_footprint.bbox;

    for ee_pad in ee_footprint.pads {
        let is_smd = ee_pad.hole_radius == 0.0;

        ki_pads.push(FpPad {
            number: ee_pad.number,
            pad_type: if is_smd {
                "smd".to_string()
            } else {
                "thru_hole".to_string()
            },
            shape: map_shape(&ee_pad.shape),
            // Shift position relative to the bounding box origin and convert units
            pos: (
                ee_to_mm(ee_pad.center_x - bbox_x),
                ee_to_mm(-(ee_pad.center_y - bbox_y)),
            ),
            size: (ee_to_mm(ee_pad.width), ee_to_mm(ee_pad.height)),
            layers: map_layer(ee_pad.layer_id, is_smd),
            rotation: -ee_pad.rotation, // KiCad rotation is often inverted
        });
    }

    // NOTE: Tracks are not yet converted to pads, just stored. A full implementation would create KiLines.
    // For now we will ignore them, but the parsing is done.

    let mut ki_texts = Vec::new();
    for ee_text in ee_footprint.texts {
        let (text_type, text) = match ee_text.text_type.as_str() {
            "P" => ("value".to_string(), ee_footprint.info.name.clone()),
            "N" => ("reference".to_string(), "REF**".to_string()),
            _ => ("user".to_string(), ee_text.text),
        };
        ki_texts.push(FpText {
            text_type,
            text,
            pos: (
                ee_to_mm(ee_text.center_x - bbox_x),
                ee_to_mm(-(ee_text.center_y - bbox_y)),
            ),
            layer: map_layer(ee_text.layer_id, true)
                .get(0)
                .unwrap_or(&"F.Fab".to_string())
                .clone(),
        });
    }

    Ok(KiFootprint {
        name: ee_footprint.info.name,
        pads: ki_pads,
        texts: ki_texts,
        model_3d: ki_model,
    })
}
/// Converts an EasyEDA 3D model (with raw OBJ data) to a KiCad 3D model (WRL).
pub fn convert_3d_model(mut ee_model: Ee3dModel) -> Result<Ki3dModel> {
    let wrl_data = if let Some(obj_data) = &ee_model.raw_obj {
        // --- Functional but simplified OBJ to WRL converter ---
        let mut vertices = Vec::new();
        let mut faces = Vec::new();

        for line in obj_data.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            match parts[0] {
                "v" => {
                    // Vertex
                    if parts.len() >= 4 {
                        let x: f32 = parts[1].parse().unwrap_or(0.0);
                        let y: f32 = parts[2].parse().unwrap_or(0.0);
                        let z: f32 = parts[3].parse().unwrap_or(0.0);
                        // EasyEDA OBJ seems to be in inches*10. Convert to mm for KiCad.
                        vertices.push(Vec3::new(x, y, z) * 2.54);
                    }
                }
                "f" => {
                    // Face
                    if parts.len() >= 4 {
                        // OBJ faces are 1-indexed. We need 0-indexed.
                        // Format is f v1//vn1 v2//vn2 v3//vn3
                        let face_indices: Vec<usize> = parts[1..]
                            .iter()
                            .map(|p| {
                                p.split('/')
                                    .next()
                                    .unwrap_or("1")
                                    .parse::<usize>()
                                    .unwrap_or(1)
                                    - 1
                            })
                            .collect();
                        faces.push(face_indices);
                    }
                }
                _ => {} // Ignore other lines (materials, normals, etc. for now)
            }
        }

        let mut wrl = String::new();
        wrl.push_str("#VRML V2.0 utf8\n");
        wrl.push_str("Shape {\n");
        wrl.push_str("  appearance Appearance {\n");
        wrl.push_str("    material Material { diffuseColor 0.5 0.5 0.5 }\n"); // Default grey
        wrl.push_str("  }\n");
        wrl.push_str("  geometry IndexedFaceSet {\n");
        wrl.push_str("    coord Coordinate {\n");
        wrl.push_str("      point [\n");
        for v in &vertices {
            wrl.push_str(&format!("        {:.4} {:.4} {:.4},\n", v.x, v.y, v.z));
        }
        wrl.push_str("      ]\n");
        wrl.push_str("    }\n");
        wrl.push_str("    coordIndex [\n");
        for f in &faces {
            let face_str = f
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<String>>()
                .join(", ");
            wrl.push_str(&format!("      {}, -1,\n", face_str));
        }
        wrl.push_str("    ]\n");
        wrl.push_str("  }\n");
        wrl.push_str("}\n");

        Some(wrl)
    } else {
        None
    };

    Ok(Ki3dModel {
        name: ee_model.name,
        wrl_data,
        step_data: ee_model.step.take(),
        offset: Vec3::ZERO,
        scale: Vec3::ONE,
        rotate: Vec3::ZERO,
    })
}
