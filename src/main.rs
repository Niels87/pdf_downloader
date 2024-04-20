#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use color_eyre::{eyre::Result, owo_colors::OwoColorize};
use std::{error::Error, fs::File, io::{copy, Write}, path};
use downloader::get_urls_from_dataframe;
use reqwest::Client;
use tokio::*;
mod xlsx_reader;
use xlsx_reader::*;
mod downloader;
mod app;
use app::*;
use color_eyre::install;
use eframe::egui::{self, viewport};

// #[tokio::main]
// async fn main() -> Result<()> {
//     color_eyre::install()?;

//     let file_path = "C:/Users/KOM/dev/rust/pdf_downloader/data/test_file_short.xlsx";

//     let df = read_xlsx(file_path)?;

//     let client_builder = Client::builder();
//     let client = client_builder
//         .user_agent(APP_USER_AGENT)
//         .build()?;

//     let downloader = downloader::Downloader::new(
//         client, 
//         df, 
//         "C:/Users/KOM/dev/rust/pdf_downloader/data/downloaded_files/"
//     );

//     downloader.download_all().await?;



//     Ok(())

// }


use eframe;

use app::DownloaderApp;

fn main() -> Result<()> {
    color_eyre::install()?;
    

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default(),
        centered: true,
        // .with_icon(
        //     // NOTE: Adding an icon is optional
        //     eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
        //         .unwrap(),
        // ),
    ..Default::default()
    };
        

    let _ = eframe::run_native(
        "Downloader Application", 
        native_options,
        Box::new(|cc| Box::new(DownloaderApp::new(cc)))
    );   
    Ok(())
}