// src/converter.rs

use crate::{easyeda_models::*, error::Result, kicad_models::*};
use glam::Vec3;

/// Converts an EasyEDA symbol to a KiCad symbol.
///
/// Handles conversion of pins, rectangles and other symbol elements while maintaining correct positioning.
pub fn convert_symbol(ee_symbol: EeSymbol) -> Result<KiSymbol> {
    let (bbox_x, bbox_y) = ee_symbol.bbox;
    let mut raw_pins = Vec::new();
    let mut raw_rects = Vec::new();

    for ee_pin in &ee_symbol.pins {
        raw_pins.push((
            ee_to_mm(ee_pin.pos_x - bbox_x),
            ee_to_mm(-(ee_pin.pos_y - bbox_y)),
        ));
    }
    for ee_rect in &ee_symbol.rectangles {
        let start_x = ee_to_mm(ee_rect.x - bbox_x);
        let start_y = ee_to_mm(-(ee_rect.y - bbox_y));
        let end_x = start_x + ee_to_mm(ee_rect.width);
        let end_y = start_y - ee_to_mm(ee_rect.height);
        raw_rects.push(((start_x, start_y), (end_x, end_y)));
    }

    // Calculate bounding box
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    for &(x, y) in &raw_pins {
        if x < min_x {
            min_x = x;
        }
        if x > max_x {
            max_x = x;
        }
        if y < min_y {
            min_y = y;
        }
        if y > max_y {
            max_y = y;
        }
    }
    for &((sx, sy), (ex, ey)) in &raw_rects {
        for &(x, y) in &[(sx, sy), (ex, ey)] {
            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        }
    }
    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;

    let mut ki_pins = Vec::new();
    for (ee_pin, &(x, y)) in ee_symbol.pins.iter().zip(raw_pins.iter()) {
        ki_pins.push(KiSymbolPin {
            name: ee_pin.name.clone(),
            number: ee_pin.number.clone(),
            pin_type: map_pin_type(&ee_pin.pin_type),
            length: ee_to_mm(ee_pin.pin_length),
            pos: (x - center_x, y - center_y),
            rotation: (ee_pin.rotation + 180) % 360,
        });
    }
    let mut ki_rects = Vec::new();
    for (ee_rect, &((sx, sy), (ex, ey))) in ee_symbol.rectangles.iter().zip(raw_rects.iter()) {
        ki_rects.push(KiSymbolRect {
            start: (sx - center_x, sy - center_y),
            end: (ex - center_x, ey - center_y),
        });
    }

    Ok(KiSymbol {
        name: ee_symbol.info.name,
        reference: ee_symbol.info.prefix,
        footprint: ee_symbol.info.package.unwrap_or_default(),
        datasheet: ee_symbol.info.datasheet.unwrap_or_default(),
        lcsc_part: ee_symbol.info.lcsc_id,
        pins: ki_pins,
        rectangles: ki_rects,
        is_extended: ee_symbol.info.is_extended,
    })
}

/// Converts an EasyEDA footprint to a KiCad footprint. (Now implemented)
/// Converts EasyEDA units to millimeters.
///
/// EasyEDA uses units that are 1/0.254 mm, this converts to standard millimeters.
fn ee_to_mm(val: f32) -> f32 {
    val * 0.254
}

/// Maps EasyEDA layer IDs to KiCad layer names.
fn map_layer(layer_id: i32, is_smd: bool) -> Vec<String> {
    // For through-hole pads, always use *.Cu and *.Mask regardless of layer_id
    if !is_smd {
        return vec!["*.Cu".to_string(), "*.Mask".to_string()];
    }

    // For SMD pads, use the appropriate layer mapping
    match layer_id {
        1 => vec![
            "F.Cu".to_string(),
            "F.Paste".to_string(),
            "F.Mask".to_string(),
        ], // Top Layer
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
///
/// Converts string shape names from EasyEDA format to KiCad's FpShape enum.
fn map_shape(shape: &str) -> FpShape {
    match shape {
        "ELLIPSE" => FpShape::Circle,
        "RECT" => FpShape::Rect,
        "OVAL" => FpShape::Oval,
        _ => FpShape::Rect, // Default fallback
    }
}
/// Maps EasyEDA pin types to KiCad pin types.
///
/// Converts EasyEDA's numeric pin type codes to KiCad's pin type enum.
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
///
/// Handles conversion of pads, text elements, and 3D model references while maintaining
/// correct positioning and scaling.
pub fn convert_footprint(
    ee_footprint: EeFootprint,
    ki_model: Option<Ki3dModel>,
) -> Result<KiFootprint> {
    let mut ki_pads = Vec::new();
    let (bbox_x, bbox_y) = ee_footprint.bbox;

    let mut raw_pad_pos = Vec::new();
    for ee_pad in &ee_footprint.pads {
        raw_pad_pos.push((
            ee_to_mm(ee_pad.center_x - bbox_x),
            ee_to_mm(ee_pad.center_y - bbox_y),
        ));
    }
    let mut raw_text_pos = Vec::new();
    for ee_text in &ee_footprint.texts {
        raw_text_pos.push((
            ee_to_mm(ee_text.center_x - bbox_x),
            ee_to_mm(ee_text.center_y - bbox_y),
        ));
    }
    // Calculate center
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut count = 0.0;
    for &(x, y) in &raw_pad_pos {
        sum_x += x;
        sum_y += y;
        count += 1.0;
    }
    for &(x, y) in &raw_text_pos {
        sum_x += x;
        sum_y += y;
        count += 1.0;
    }
    let center_x = if count > 0.0 { sum_x / count } else { 0.0 };
    let center_y = if count > 0.0 { sum_y / count } else { 0.0 };

    for (idx, (ee_pad, &(x, y))) in ee_footprint.pads.iter().zip(raw_pad_pos.iter()).enumerate() {
        let is_smd = ee_pad.hole_radius == 0.0;
        // Use provided number, but fallback to a deterministic index-based number if empty
        let pad_number = if ee_pad.number.trim().is_empty() {
            (idx + 1).to_string()
        } else {
            ee_pad.number.clone()
        };
        ki_pads.push(FpPad {
            number: pad_number,
            pad_type: if is_smd {
                "smd".to_string()
            } else {
                "thru_hole".to_string()
            },
            shape: map_shape(&ee_pad.shape),
            pos: (x - center_x, y - center_y),
            size: (ee_to_mm(ee_pad.width), ee_to_mm(ee_pad.height)),
            layers: map_layer(ee_pad.layer_id, is_smd),
            rotation: -ee_pad.rotation,
            drill: if is_smd {
                None
            } else {
                // EeFootprintPad.hole_radius is a radius in EasyEDA units; convert to diameter in mm
                Some(ee_to_mm(ee_pad.hole_radius * 2.0))
            },
        });
    }

    // NOTE: Tracks are not yet converted to pads, just stored. A full implementation would create KiLines.
    // For now we will ignore them, but the parsing is done.

    let mut ki_texts = Vec::new();
    for (ee_text, &(x, y)) in ee_footprint.texts.iter().zip(raw_text_pos.iter()) {
        let (text_type, text) = match ee_text.text_type.as_str() {
            "P" => ("value".to_string(), ee_footprint.info.name.clone()),
            "N" => ("reference".to_string(), "REF**".to_string()),
            _ => ("user".to_string(), ee_text.text.clone()),
        };
        ki_texts.push(FpText {
            text_type,
            text,
            pos: (x - center_x, y - center_y),
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
/// Converts an EasyEDA 3D model (with raw OBJ data) to a KiCad 3D model (VRML).
///
/// Converts vertices and faces from OBJ format to VRML format, applying appropriate scaling
/// and maintaining all geometric information.
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
                        // EasyEDA OBJ seems to be in inches*10. Convert to mm and scale down by 10
                        vertices.push(Vec3::new(x, y, z) * 0.254 * 1.55); // 1.55 is a scaler that seems to fix scaling values
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
