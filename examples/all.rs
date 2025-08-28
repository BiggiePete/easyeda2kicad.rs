use easyeda2kicad_rs::import_component;
use std::{path::Path, time::Instant};

#[tokio::main]
async fn main() {
    println!("Hello World!");
    let lcsc_ids = vec![
        "C22392644",
        "C324124",
        "C8952",
        "C2040",
        "C5659",
        "C2057889",
        "C42371095",
        "C5452091",
        "C209907",
    ]; // Example LCSC IDs

    let start_time = Instant::now();
    for lcsc_id in &lcsc_ids {
        let entry_start_time = Instant::now();
        match import_component(lcsc_id, Path::new("example_lib")).await {
            Ok(component) => println!("Imported component: {:?}", component),
            Err(e) => eprintln!("Error importing component: {}", e),
        }

        println!("Got entry in : {:?}\n\n\n", entry_start_time.elapsed());
    }
    println!(
        "Got {:?} Entries in : {:?}",
        &lcsc_ids.len(),
        start_time.elapsed()
    );
}
