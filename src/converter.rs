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
/// correct positioning and scaling. Now supports oval/slot holes.
pub fn convert_footprint(
    ee_footprint: EeFootprint,
    ki_model: Option<Ki3dModel>,
) -> Result<KiFootprint> {
    let mut ki_pads = Vec::new();
    let mut ki_graphics = Vec::new();
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

    // Calculate centroid (center of the component) based on pads
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut count = 0.0;
    for &(x, y) in &raw_pad_pos {
        sum_x += x;
        sum_y += y;
        count += 1.0;
    }
    // Fallback to text position if no pads exist (e.g. logo, fiducial)
    if count == 0.0 {
        for &(x, y) in &raw_text_pos {
            sum_x += x;
            sum_y += y;
            count += 1.0;
        }
    }

    let center_x = if count > 0.0 { sum_x / count } else { 0.0 };
    let center_y = if count > 0.0 { sum_y / count } else { 0.0 };

    // --- PADS ---
    for (idx, (ee_pad, &(x, y))) in ee_footprint.pads.iter().zip(raw_pad_pos.iter()).enumerate() {
        let is_smd = ee_pad.hole_radius == 0.0 && ee_pad.hole_length == 0.0;
        let pad_number = if ee_pad.number.trim().is_empty() {
            (idx + 1).to_string()
        } else {
            ee_pad.number.clone()
        };

        let (drill, drill_oval) = if is_smd {
            (None, None)
        } else if ee_pad.hole_length > 0.0 {
            let drill_width = ee_to_mm(ee_pad.hole_radius * 2.0);
            let drill_height = ee_to_mm(ee_pad.hole_length);
            (None, Some((drill_width, drill_height)))
        } else {
            let drill_dia = ee_to_mm(ee_pad.hole_radius * 2.0);
            (Some(drill_dia), None)
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
            drill,
            drill_oval,
        });
    }

    // --- TRACKS (Lines/Polygons) ---
    // This provides the body outline on silkscreen/fab layers
    for track in &ee_footprint.tracks {
        let layers = map_layer(track.layer_id, true);
        let layer_name = &layers[0];

        // Skip copper tracks (Layer 1/2) unless you specifically want net ties.
        // Usually footprint graphics are on Silk(3/4), Fab(13), or Doc(15).
        let is_graphic_layer = layer_name.contains("Silk")
            || layer_name.contains("Fab")
            || layer_name.contains("Dwgs");

        if is_graphic_layer && track.points.len() >= 2 {
            let width = ee_to_mm(track.stroke_width);

            for i in 0..track.points.len() - 1 {
                let (x1, y1) = track.points[i];
                let (x2, y2) = track.points[i + 1];

                let start_x = ee_to_mm(x1 - bbox_x) - center_x;
                let start_y = ee_to_mm(y1 - bbox_y) - center_y;
                let end_x = ee_to_mm(x2 - bbox_x) - center_x;
                let end_y = ee_to_mm(y2 - bbox_y) - center_y;

                // SANITY CHECK: Distance
                // If a line is > 150mm away from the center, it's garbage (e.g. frame border).
                if start_x.abs() > 150.0 || start_y.abs() > 150.0 {
                    continue;
                }

                ki_graphics.push(FpGraphic {
                    layer: layer_name.clone(),
                    width,
                    graphic_type: FpGraphicType::Line {
                        start: (start_x, start_y),
                        end: (end_x, end_y),
                    },
                });
            }
        }
    }

    // --- CIRCLES ---
    for circle in &ee_footprint.circles {
        // FILTER: Ignore circles on Fab/Doc layers (13, 15).
        // EasyEDA often puts "Pick and Place Origin" or "Collision Radii" here which are massive.
        // We only want Silkscreen (3, 4) or Copper (1, 2).
        if circle.layer_id != 1
            && circle.layer_id != 2
            && circle.layer_id != 3
            && circle.layer_id != 4
        {
            continue;
        }

        let layers = map_layer(circle.layer_id, true);
        let layer_name = &layers[0];

        let cx = ee_to_mm(circle.center_x - bbox_x) - center_x;
        let cy = ee_to_mm(circle.center_y - bbox_y) - center_y;
        let radius = ee_to_mm(circle.radius);

        // SANITY CHECK: Distance
        // If the circle center is miles away, drop it.
        if cx.abs() > 150.0 || cy.abs() > 150.0 {
            continue;
        }

        // SANITY CHECK: Size
        // If the circle is massive (>50mm radius), it's likely a collision courtyard, not a graphic.
        if radius > 50.0 {
            continue;
        }

        let end_x = cx + radius;
        let end_y = cy;

        ki_graphics.push(FpGraphic {
            layer: layer_name.clone(),
            width: ee_to_mm(circle.stroke_width),
            graphic_type: FpGraphicType::Circle {
                center: (cx, cy),
                end: (end_x, end_y),
            },
        });
    }

    // --- TEXTS ---
    let mut ki_texts = Vec::new();
    for (ee_text, &(x, y)) in ee_footprint.texts.iter().zip(raw_text_pos.iter()) {
        let (text_type, text) = match ee_text.text_type.as_str() {
            "P" => ("value".to_string(), ee_footprint.info.name.clone()),
            "N" => ("reference".to_string(), "REF**".to_string()),
            _ => ("user".to_string(), ee_text.text.clone()),
        };

        // Standardize layers for text
        let mut layer = map_layer(ee_text.layer_id, true)
            .get(0)
            .unwrap_or(&"F.Fab".to_string())
            .clone();

        // Ensure Reference and Value are on reasonable layers
        if text_type == "reference" {
            layer = "F.SilkS".to_string();
        }
        if text_type == "value" {
            layer = "F.Fab".to_string();
        }

        ki_texts.push(FpText {
            text_type,
            text,
            pos: (x - center_x, y - center_y),
            layer,
        });
    }

    // automatic marker for Pin1
    let pin1 = ki_pads
        .iter()
        .find(|p| p.number == "1")
        .or_else(|| ki_pads.iter().find(|p| p.number == "A1"));

    if let Some(p1) = pin1 {
        // Properties
        let marker_radius = 0.25; // mm
        let gap = 0.5; // mm clearance from the pad edge

        let (px, py) = p1.pos;
        let (sx, sy) = p1.size;

        // Determine direction to push the marker relative to the component center (0,0).
        // If px is negative (left side), push further left (-1.0).
        // If px is positive (right side), push further right (1.0).
        // Use a small epsilon to handle pads exactly on the centerline.
        let dir_x = if px < -0.01 {
            -1.0
        } else if px > 0.01 {
            1.0
        } else {
            -1.0
        }; // Default left for center vertical
        let dir_y = if py < -0.01 {
            -1.0
        } else if py > 0.01 {
            1.0
        } else {
            -1.0
        }; // Default top for center horizontal

        // Calculate position: Pad Center + (Half Size + Gap) * Direction
        // We use the larger dimension of the pad to ensure we clear it regardless of rotation
        let clearance_x = (sx / 2.0) + gap;
        let clearance_y = (sy / 2.0) + gap;

        // Place the dot.
        // We prioritize placing it along the longest axis of distance from center to corners
        // to put it nicely in the corner of the IC.
        let dot_x = px + (clearance_x * dir_x);
        let dot_y = py + (clearance_y * dir_y);

        ki_graphics.push(FpGraphic {
            layer: "F.SilkS".to_string(),
            width: 0.15, // Thickness of the circle line
            graphic_type: FpGraphicType::Circle {
                center: (dot_x, dot_y),
                // KiCad circle is defined by Center + Point on Edge.
                // We add the radius to X to define that edge point.
                end: (dot_x + marker_radius, dot_y),
            },
        });
    }

    Ok(KiFootprint {
        name: ee_footprint.info.name,
        pads: ki_pads,
        texts: ki_texts,
        graphics: ki_graphics,
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
