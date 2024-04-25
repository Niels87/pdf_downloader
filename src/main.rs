#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eyre::*;
mod xlsx_reader;
mod downloader;
mod app;
mod network_monitor;
use eframe::egui;
use app::DownloaderApp;

//#[tokio::main]
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    //eyre::install()?;
    

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default(),
        centered: true,
    ..Default::default()
    };
        

    let _ = eframe::run_native(
        "Downloader Application", 
        native_options,
        Box::new(|cc| Box::new(DownloaderApp::new(cc)))
    );   
    Ok(())
}
// #[cfg(target_arch = "wasm32")]
// fn main() {
//     //eyre::install()?;
    

//     let web_options = eframe::WebOptions::default();
    
//     wasm_bindgen_futures::spawn_local(async {
//         eframe::WebRunner::new()
//             .start(
//                 "the_canvas_id", // hardcode it
//                 web_options,
//                 Box::new(|cc| Box::new(DownloaderApp::new(cc))),
//             )
//             .await
//             .expect("failed to start eframe");

// });
// }

