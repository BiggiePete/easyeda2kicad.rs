// src/importer.rs

use crate::easyeda_models::*;
use crate::error::{Error, Result};
use serde::Deserialize;
use serde_json::Value;

/// Parses the JSON value from the API into an EeSymbol.
/// A real implementation would parse the complex `dataStr` field.
pub fn import_symbol(data: &Value) -> Result<EeSymbol> {
    let data_str = &data["dataStr"];
    let c_para = &data_str["head"]["c_para"];

    let info = EeSymbolInfo {
        name: c_para["name"].as_str().unwrap_or("Unknown").to_string(),
        prefix: c_para["pre"].as_str().unwrap_or("U").to_string(),
        package: c_para["package"].as_str().map(String::from),
        datasheet: data["lcsc"]["url"].as_str().map(String::from),
        lcsc_id: data["lcsc"]["number"].as_str().map(String::from),
        is_extended: c_para["JLCPCB Part Class"]
            .as_str()
            .eq(&Some("Extended Part")),
    };

    let bbox_x = data_str["head"]["x"]
        .as_str()
        .unwrap_or("0")
        .parse::<f32>()
        .unwrap_or(0.0);
    let bbox_y = data_str["head"]["y"]
        .as_str()
        .unwrap_or("0")
        .parse::<f32>()
        .unwrap_or(0.0);

    let mut pins = Vec::new();
    let mut rectangles = Vec::new();

    let shapes = data_str["shape"]
        .as_array()
        .ok_or_else(|| Error::MissingData("Symbol shape data is missing".to_string()))?;

    for shape_val in shapes {
        let shape_str = shape_val.as_str().unwrap_or("");

        // Symbol pins are more complex, delimited by "^^"
        if shape_str.starts_with('P') && shape_str.contains("^^") {
            let segments: Vec<&str> = shape_str.split("^^").collect();
            if segments.len() >= 4 {
                let settings = parse_raw_line(segments[0]);
                let path = parse_raw_line(segments[2]);
                let name_data = parse_raw_line(segments[3]);

                if settings.len() > 7 && name_data.len() > 5 && path.len() > 1 {
                    let path_commands: Vec<&str> = path[1].split_whitespace().collect();
                    let pin_length = path_commands.last().unwrap_or(&"0").parse().unwrap_or(10.0);

                    pins.push(EeSymbolPin {
                        number: settings[3].to_string(),
                        name: name_data[4].to_string(),
                        pos_x: settings[4].parse().unwrap_or(0.0),
                        pos_y: settings[5].parse().unwrap_or(0.0),
                        rotation: settings[6].parse().unwrap_or(0),
                        pin_type: settings[2].to_string(),
                        pin_length,
                    });
                }
            }
        } else {
            // Handle other shapes like rectangles
            let fields = parse_raw_line(shape_str);
            if fields.is_empty() {
                continue;
            }
            match fields[0] {
                "R" => {
                    // Rectangle
                    if fields.len() > 6 {
                        rectangles.push(EeSymbolRectangle {
                            x: fields[1].parse().unwrap_or(0.0),
                            y: fields[2].parse().unwrap_or(0.0),
                            width: fields[5].parse().unwrap_or(0.0),
                            height: fields[6].parse().unwrap_or(0.0),
                        });
                    }
                }
                _ => { /* Ignore polylines, circles etc for now */ }
            }
        }
    }

    Ok(EeSymbol {
        info,
        bbox: (bbox_x, bbox_y),
        pins,
        rectangles,
    })
}

// Helper structs for deserializing the nested JSON inside the SVGNODE string.
#[derive(Deserialize, Debug)]
struct SvgNode {
    attrs: SvgNodeAttrs,
}

#[derive(Deserialize, Debug)]
struct SvgNodeAttrs {
    uuid: String,
    title: String,
    // Add other fields like c_origin, c_rotation if needed for placement
}

/// Extracts 3D model info by correctly parsing the SVGNODE from the footprint shape data.
pub fn import_3d_model_info(data: &Value) -> Result<Option<Ee3dModel>> {
    let shapes = data["packageDetail"]["dataStr"]["shape"]
        .as_array()
        .ok_or_else(|| {
            Error::MissingData("Footprint shape data is missing or not an array".to_string())
        })?;

    for shape_value in shapes {
        if let Some(shape_str) = shape_value.as_str() {
            // The line we are looking for starts with "SVGNODE~"
            if shape_str.starts_with("SVGNODE~") {
                // Split the string into "SVGNODE" and the JSON part.
                // We use `split_once` to be safe.
                if let Some((_, json_part)) = shape_str.split_once('~') {
                    // The json_part is a string containing JSON, so we parse it.
                    let svg_node: SvgNode = serde_json::from_str(json_part)?;

                    // We found it! Now we can build our Ee3dModel and return.
                    return Ok(Some(Ee3dModel {
                        name: svg_node.attrs.title,
                        uuid: svg_node.attrs.uuid,
                        raw_obj: None,
                        step: None,
                    }));
                }
            }
        }
    }

    // If we loop through all shapes and don't find an SVGNODE, there is no model.
    Ok(None)
}

fn parse_raw_line(line: &str) -> Vec<&str> {
    line.split('~').collect()
}

/// Parses the detailed footprint data from the `dataStr` field.
pub fn import_footprint(data: &Value) -> Result<EeFootprint> {
    let data_str = &data["packageDetail"]["dataStr"];
    let info = EeFootprintInfo {
        name: data["packageDetail"]["title"]
            .as_str()
            .unwrap_or("UnknownFootprint")
            .to_string(),
    };

    let bbox_x = data_str["head"]["x"]
        .as_str()
        .unwrap_or("0")
        .parse::<f32>()
        .unwrap_or(0.0);
    let bbox_y = data_str["head"]["y"]
        .as_str()
        .unwrap_or("0")
        .parse::<f32>()
        .unwrap_or(0.0);

    let mut pads = Vec::new();
    let mut tracks = Vec::new();
    let mut texts = Vec::new();

    let mut circles = Vec::new();
    let mut arcs = Vec::new();

    let shapes = data_str["shape"]
        .as_array()
        .ok_or_else(|| Error::MissingData("Footprint shape data is missing".to_string()))?;

    for shape_val in shapes {
        let shape_str = shape_val.as_str().unwrap_or("");
        let fields = parse_raw_line(shape_str);
        if fields.is_empty() {
            continue;
        }

        match fields[0] {
            "PAD" => {
                // PAD format from EasyEDA:
                // [0]PAD [1]shape [2]x [3]y [4]width [5]height [6]layer [7]net [8]number
                // [9]hole_radius [10]points [11]rotation [12]id [13]hole_length ...
                if fields.len() > 11 {
                    let hole_radius = fields[9].parse().unwrap_or(0.0);

                    // FIX: Check field 13 first.
                    // In modern EasyEDA, field 12 is the ID (UUID), and field 13 is the hole length.
                    let mut hole_length = if fields.len() > 13 {
                        fields[13].parse::<f32>().unwrap_or(0.0)
                    } else {
                        0.0
                    };

                    // Fallback for very old formats where field 12 might have been the length.
                    // (If field 12 is a UUID, parse fails and returns 0.0, so this is safe)
                    if hole_length == 0.0 && fields.len() > 12 {
                        let val = fields[12].parse::<f32>().unwrap_or(0.0);
                        // Only accept it if it looks like a length (not an ID)
                        if val > 0.0 {
                            hole_length = val;
                        }
                    }

                    pads.push(EeFootprintPad {
                        shape: fields[1].to_string(),
                        center_x: fields[2].parse().unwrap_or(0.0),
                        center_y: fields[3].parse().unwrap_or(0.0),
                        width: fields[4].parse().unwrap_or(0.0),
                        height: fields[5].parse().unwrap_or(0.0),
                        layer_id: fields[6].parse().unwrap_or(0),
                        number: fields[8].to_string(),
                        hole_radius,
                        hole_length, // This will now be populated correctly
                        rotation: fields[11].parse().unwrap_or(0.0),
                    });
                }
            }
            "TRACK" => {
                if fields.len() > 4 {
                    let points_str: Vec<&str> = fields[4].split(' ').collect();
                    let mut points = Vec::new();
                    for i in (0..points_str.len()).step_by(2) {
                        if i + 1 < points_str.len() {
                            let x = points_str[i].parse().unwrap_or(0.0);
                            let y = points_str[i + 1].parse().unwrap_or(0.0);
                            points.push((x, y));
                        }
                    }
                    tracks.push(EeFootprintTrack {
                        stroke_width: fields[1].parse().unwrap_or(0.0),
                        layer_id: fields[2].parse().unwrap_or(0),
                        points,
                    });
                }
            }
            "TEXT" => {
                if fields.len() > 10 {
                    texts.push(EeFootprintText {
                        text_type: fields[1].to_string(),
                        center_x: fields[2].parse().unwrap_or(0.0),
                        center_y: fields[3].parse().unwrap_or(0.0),
                        rotation: fields[5].parse().unwrap_or(0.0),
                        layer_id: fields[7].parse().unwrap_or(0),
                        text: fields[10].to_string(),
                    });
                }
            }
            "CIRCLE" => {
                // Format: CIRCLE~layer~width~cx~cy~radius~id
                if fields.len() > 5 {
                    circles.push(EeFootprintCircle {
                        layer_id: fields[1].parse().unwrap_or(0),
                        stroke_width: fields[2].parse().unwrap_or(0.1),
                        center_x: fields[3].parse().unwrap_or(0.0),
                        center_y: fields[4].parse().unwrap_or(0.0),
                        radius: fields[5].parse().unwrap_or(0.0),
                    });
                }
            }
            // Add ARC Parsing
            "ARC" => {
                // Format: ARC~layer~width~pathString~id
                if fields.len() > 3 {
                    arcs.push(EeFootprintArc {
                        layer_id: fields[1].parse().unwrap_or(0),
                        stroke_width: fields[2].parse().unwrap_or(0.1),
                        path: fields[3].to_string(),
                    });
                }
            }
            _ => { /* Silently ignore unsupported shapes */ }
        }
    }

    Ok(EeFootprint {
        info,
        bbox: (bbox_x, bbox_y),
        pads,
        tracks,
        texts,
        circles, // Add to struct
        arcs,    // Add to struct
    })
}
