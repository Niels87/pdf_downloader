use eframe::egui::Color32;
use eframe::Frame;
use eframe::egui;
use polars::frame::DataFrame;
use serde;
use tokio::task::JoinHandle;
use crate::downloader;
use crate::downloader::*;
use reqwest::Client;
use crate::xlsx_reader::*;
use color_eyre::eyre::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::result::Result::Ok as std_Ok;
use std::str::FromStr;
use crate::xlsx_reader::read_xlsx;
use egui_file_dialog::*;
use egui::ProgressBar;
use std::sync::{Arc, Mutex};


/// We derive Deserialize/Serialize so we can persist app state on shutdown.
//#[derive(serde::Deserialize, serde::Serialize)]
//#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct DownloaderApp {
    downloader: Option<Downloader>,
    //text_inputs: TextDisplays,
    dataframe: Option<DataFrame>,
    url_columns: Vec<String>,
    file_name_column: Option<String>,
    xlsx_file: Option<PathBuf>,
    output_folder: Option<PathBuf>,
    //log_file: String,
    ui: AppUI,
    download_state: AppStatus,
    //active_downloads: Vec<JoinHandle<Result<String, Report>>>
}

// pub struct TextDisplays {
//     /// The .xlsx that contains the target information.
//     pub xlsx_file_status: String,
//     /// The folder where we want to store downloaded files.
//     pub output_folder_str: String,

// }

// impl Default for TextDisplays {
//     fn default() -> Self {
//         TextDisplays { 
//             xlsx_file_status: "Input the path to the xlsx file".to_string(), 
//             output_folder_str: "Input where you want to store downloaded files".to_string()
//         }
//     }
// }

pub struct AppUI {
    choose_xlsx: FileDialog,
    choose_output_folder: FileDialog,
    //choose_logfile: FileDialog,
    progress_bars: Option<HashMap<String, ProgressBar>>,
}

impl Default for AppUI {
    fn default() -> Self {
        AppUI { 
            choose_xlsx: FileDialog::new(), 
            choose_output_folder: FileDialog::new(),
            //choose_logfile: FileDialog::new(),
            progress_bars: None,
        }
    }
    
    
}

// impl Default for DownloaderApp {
//     fn default() -> Self {
//         Self {
//         }
//     }
// }

impl DownloaderApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        // if let Some(storage) = cc.storage {
        //     return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        // }
        // Default::default()

        let app = DownloaderApp { 
            downloader: None,
            //text_inputs: TextDisplays::default(),
            dataframe: None,
            url_columns: vec!["Pdf_URL".to_string(), "Report Html Address".to_string()],
            file_name_column: Some("BRnum".to_string()),
            xlsx_file: None, //Some(PathBuf::from("C:/Users/KOM/dev/rust/pdf_downloader/data/test_file_short.xlsx")),
            output_folder: None, //Some(PathBuf::from("C:/Users/KOM/dev/rust/pdf_downloader/data/downloaded_files/")),
            //log_file: "download_log.csv".to_string(),
            ui: AppUI::default(),
            download_state: AppStatus::NotStarted,
            //active_downloads: Vec::new(),
        };
        app
    }

    pub fn on_build_downloader_button_clicked(&mut self) -> Result<()> {
        self.downloader = Some(build_downloader(
            self.dataframe.clone().unwrap(),
            self.output_folder.clone().unwrap(),
            self.file_name_column.clone().unwrap(),
            self.url_columns.clone(),
        )?);
        Ok(())
    }

    pub fn on_read_xlsx_button_clicked(&mut self) {
        if let Some(path) = &self.xlsx_file {
            if let std_Ok(df) = read_xlsx(path) {
                self.dataframe = Some(df);
            }        
        }    
    }

    pub fn on_download_all_button_clicked(&mut self) -> Result<()> {
        
        match &mut self.downloader {
            Some(d) => {
                self.download_state = AppStatus::Downloading;
                let handles = d.download_all()?;
            },
            None => ()
        }
        Ok(())
    }
}

impl eframe::App for DownloaderApp {
    /// Called by the frame work to save state before shutdown.
    // fn save(&mut self, storage: &mut dyn eframe::Storage) {
    //     eframe::set_value(storage, eframe::APP_KEY, self);
    // }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        ctx.request_repaint();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {

                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.add_space(16.0);
                egui::widgets::global_dark_light_mode_buttons(ui);                
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });

        });
        
        egui::SidePanel::right("right_panel")
            .min_width(320.0)
            .show(ctx, |ui| {
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some(downloader) = &self.downloader {
                        if let std_Ok(p) = downloader.progress.lock() {
                            for dl in p.iter() {
                                if !dl.1.finished {
                                    ui.label(format!("{}", dl.0));
                                    ui.add(
                                    egui::ProgressBar::new(dl.1.downloaded as f32 / dl.1.size as f32)
                                        .show_percentage()
                                        .fill(Color32::DARK_GRAY)
                                    );
                                }
                            }
                        }
                    }
                })
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            
            if ui.button("Open file").clicked() {
                        
                self.ui.choose_xlsx.select_file();
            }   
            if let Some(path) = self.ui.choose_xlsx.update(ctx).selected() {
                
                self.xlsx_file = Some(path.to_path_buf()); 
            }         
            
            ui.label(
                match &self.xlsx_file {
                    Some(f) => format!("File: {}", f.display()),
                    None => "No file loaded".to_string()
                }    
            );
            if ui.button("Choose folder").clicked() {
                self.ui.choose_output_folder.select_directory()
            }
            if let Some(path) = self.ui.choose_output_folder.update(ctx).selected() {
                self.output_folder = Some(path.to_path_buf()); 
            }
            ui.label(
                match &self.output_folder {
                    Some(f) => format!("Folder: {}", f.display()),
                    None => "No folder chosen".to_string()
                }    
            );
            if ui.add_enabled(
                self.xlsx_file.is_some(), 
                egui::Button::new("Read xlsx")
            ).clicked() {
                self.on_read_xlsx_button_clicked()
            }
            if self.dataframe.is_some() {
                ui.label("xlsx read!");
            }
            // match &self.dataframe {
            //     Some(df) => {
            //         ui.horizontal(|ui| {
            //             ui.menu_button("Url columns", |menu_ui| {
            //                 for cn in df.get_column_names() {
            //                     if !self.url_columns.contains(&cn.to_string()) {
            //                         if menu_ui.button(cn).clicked() {
            //                             self.url_columns.push(cn.to_string());
            //                             menu_ui.close_menu();
            //                         }    
            //                     }
            //                 }
            //             });
            //             ui.menu_button("Name column", |menu_ui| {
            //                 for cn in df.get_column_names() {
            //                     if !self.url_columns.contains(&cn.to_string()) {
            //                         if menu_ui.button(cn).clicked() {
            //                             self.file_name_column = Some(cn.to_string());
            //                             menu_ui.close_menu();
            //                         }    
            //                     }
            //                 }
            //             });
            //         });
            //     }
            //     None => ()
            // }
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label("Filename column");
                    if let Some(file_name_column) = &self.file_name_column {
                        ui.label(file_name_column);
                    }
                });
                ui.vertical(|ui| {
                    ui.label("Url columns:");
                    for col in &self.url_columns {
                        ui.label(col);
                    }
                });
            });
            
            ui.horizontal(|ui| {
                if ui.add_enabled(
                    self.dataframe.is_some() && self.output_folder.is_some(),
                    egui::Button::new("Build downloader")
                ).clicked() {
                    let _ = self.on_build_downloader_button_clicked();
                }
                if self.downloader.is_some() {
                    ui.label(format!(" downloader built"));    
                }    
            });
            
            if ui.add_enabled(
                self.downloader.is_some(), 
                egui::Button::new("Download all files")
            ).clicked() {
                let _ = self.on_download_all_button_clicked();
            }

            // if let Some(downloader) = &self.downloader {
            //     match downloader.status {
            //         AppStatus::Downloading => { ui.label("Download in progress..."); },
            //         //AppStatus::FlushingLog => { ui.label("Flushing log file to disk..."); }
            //         AppStatus::Finished => { ui.label("Finished!"); },
            //         _ => (),
            //     }
            // }
        });
    
        fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("Powered by ");
                ui.hyperlink_to("egui", "https://github.com/emilk/egui");
                ui.label(" and ");
                ui.hyperlink_to(
                    "eframe",
                    "https://github.com/emilk/egui/tree/master/crates/eframe",
                );
                ui.label(".");
            });
        }
    }
}


