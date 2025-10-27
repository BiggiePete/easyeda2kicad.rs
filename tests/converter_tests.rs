use easyeda2kicad_rs::{
    converter::{convert_footprint, convert_symbol},
    easyeda_models::{
        EeFootprint, EeFootprintInfo, EeFootprintPad, EeSymbol, EeSymbolInfo, EeSymbolPin,
        EeSymbolRectangle,
    },
};
use std::{path::Path, time::Instant};
use tokio;

#[tokio::test]
async fn test_basic_component_import() {
    // Test importing a simple component (C2040 - 0805 capacitor)
    let test_lcsc_id = "C2040";
    let result = easyeda2kicad_rs::import_component(test_lcsc_id, Path::new("test_output")).await;
    assert!(
        result.is_ok(),
        "Failed to import basic component: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_complex_component_import() {
    // Test importing a more complex component (C324124 - STM32 MCU)
    let test_lcsc_id = "C324124";
    let result = easyeda2kicad_rs::import_component(test_lcsc_id, Path::new("test_output")).await;
    assert!(
        result.is_ok(),
        "Failed to import complex component: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_footprint_conversion() {
    // Create a simple test footprint (SMD pad)
    let ee_footprint = EeFootprint {
        info: EeFootprintInfo {
            name: "TEST_PAD".to_string(),
            ..Default::default()
        },
        bbox: (0.0, 0.0),
        pads: vec![EeFootprintPad {
            shape: "RECT".to_string(),
            center_x: 0.0,
            center_y: 0.0,
            width: 1.2,
            height: 0.6,
            layer_id: 1,
            number: "1".to_string(),
            hole_radius: 0.0,
            rotation: 0.0,
        }],
        tracks: vec![],
        texts: vec![],
    };

    let result = convert_footprint(ee_footprint, None);
    assert!(
        result.is_ok(),
        "Failed to convert simple footprint: {:?}",
        result.err()
    );

    if let Ok(kicad_fp) = result {
        assert_eq!(
            kicad_fp.pads.len(),
            1,
            "Expected one pad in converted footprint"
        );
        assert_eq!(
            kicad_fp.pads[0].number, "1",
            "Expected pad number to be '1'"
        );
    }
}

#[tokio::test]
async fn test_symbol_conversion() {
    // Create a simple test symbol (resistor)
    let ee_symbol = EeSymbol {
        info: EeSymbolInfo {
            name: "TEST_R".to_string(),
            prefix: "R".to_string(),
            package: Some("0805".to_string()),
            datasheet: None,
            lcsc_id: Some("C1234".to_string()),
            is_extended: false,
        },
        bbox: (0.0, 0.0),
        pins: vec![
            EeSymbolPin {
                number: "1".to_string(),
                name: "1".to_string(),
                pos_x: -5.0,
                pos_y: 0.0,
                rotation: 0,
                pin_type: "passive".to_string(),
                pin_length: 2.54,
            },
            EeSymbolPin {
                number: "2".to_string(),
                name: "2".to_string(),
                pos_x: 5.0,
                pos_y: 0.0,
                rotation: 180,
                pin_type: "passive".to_string(),
                pin_length: 2.54,
            },
        ],
        rectangles: vec![EeSymbolRectangle {
            x: -2.5,
            y: -1.0,
            width: 5.0,
            height: 2.0,
        }],
    };

    let result = convert_symbol(ee_symbol);
    assert!(
        result.is_ok(),
        "Failed to convert simple symbol: {:?}",
        result.err()
    );

    if let Ok(kicad_sym) = result {
        assert_eq!(
            kicad_sym.pins.len(),
            2,
            "Expected two pins in converted symbol"
        );
    }
}

#[tokio::test]
async fn test_multiple_component_import() {
    let lcsc_ids = vec!["C8952", "C2040", "C5659"];
    let start_time = Instant::now();

    for lcsc_id in &lcsc_ids {
        let result = easyeda2kicad_rs::import_component(lcsc_id, Path::new("test_output")).await;
        assert!(
            result.is_ok(),
            "Failed to import component {}: {:?}",
            lcsc_id,
            result.err()
        );
    }

    println!(
        "Imported {} components in {:?}",
        lcsc_ids.len(),
        start_time.elapsed()
    );
}

#[tokio::test]
async fn test_invalid_component_import() {
    // Test importing a non-existent component
    let test_lcsc_id = "INVALID_ID";
    let result = easyeda2kicad_rs::import_component(test_lcsc_id, Path::new("test_output")).await;
    assert!(
        result.is_err(),
        "Expected error when importing invalid component"
    );
}
