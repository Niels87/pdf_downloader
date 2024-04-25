use std::collections::{HashMap, HashSet};
use std::io::SeekFrom;
use std::path::{self, PathBuf};
use std::result::Result::Ok as std_Ok;
use std::sync::Arc;
use eframe::egui::{Align2, Color32};
use eframe::egui;
use egui_file_dialog::*;
use egui::ProgressBar;
use eyre::Result;
use sysinfo::*;
use crate::downloader::{self, *};
use crate::xlsx_reader::{get_xlsx_column_names, read_xlsx};
use crate::network_monitor;


/// We derive Deserialize/Serialize so we can persist app state on shutdown.
//#[derive(serde::Deserialize, serde::Serialize)]
//#[serde(default)] // if we add new fields, give them default values when deserializing old state

pub struct DownloaderApp {
    xlsx_file: Option<PathBuf>,
    output_folder: Option<PathBuf>,
    all_xlsx_column_names: Option<Vec<String>>,
    file_name_column: Option<String>,
    url_columns: Vec<String>,    
    //keys are the column headers, values are vectors containing column data.
    fnames_with_urls: Option<HashMap<String, Vec<String>>>,
    //log_file: String,
    downloader: Option<Arc<Downloader>>,
    ui: AppUI,
    download_state: AppStatus,
    net_tracker: network_monitor::NetTracker,
    //active_downloads: Vec<JoinHandle<Result<String, Report>>>
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
            menu_bar(ui, ctx);
        });

        egui::SidePanel::right("right_panel")
            .min_width(320.0)
            .show(ctx, |ui| {
                download_progress(ui, &self.downloader)
                              
            });
        
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });

        });
                
        egui::CentralPanel::default().show(ctx, |ui| {


            self.buttons(ui, ctx);
            

            ui.horizontal(|ui| {
                egui::CollapsingHeader::new("Failed").show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if let Some(downloader) = &self.downloader {
                            if let std_Ok(p) = downloader.progress.lock() {
                                for (fname, dp) in p.iter() {
                                    if let Some(r) = &dp.result {
                                        if let Err(e) = r {    
                                            ui.label(
                                                format!("{}: {:?}", &fname, r) 
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    });
                });
                
                egui::CollapsingHeader::new("Succesful").show(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        if let Some(downloader) = &self.downloader {
                            if let std_Ok(p) = downloader.progress.lock() {
                                for (fname, dp) in p.iter() {
                                    if let Some(r) = &dp.result {
                                        if let Ok(e) = r {    
                                            ui.label(
                                                format!("{}: {:?}", &fname, e) 
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    });
                });
                
            });

            ui.add_space(32.0);

            self.net_tracker.update();

            ui.label(format!("Mbit/s: {}", self.net_tracker.usage));
            ui.add_space(32.0);

            self.net_tracker.plot_usage_history(ui);
            
        });

        
        
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
            xlsx_file: None, //Some(PathBuf::from("C:/Users/KOM/dev/rust/pdf_downloader/data/test_file_short.xlsx")),
            output_folder: None, //Some(PathBuf::from("C:/Users/KOM/dev/rust/pdf_downloader/data/downloaded_files/")),
            all_xlsx_column_names: None,
            downloader: None,
            //text_inputs: TextDisplays::default(),
            url_columns: Vec::new(), //Some(vec!["Pdf_URL".to_string(), "Report Html Address".to_string()]),
            file_name_column: None,
            fnames_with_urls: None,
            //log_file: "download_log.csv".to_string(),
            ui: AppUI::default(),
            download_state: AppStatus::NotStarted,
            net_tracker: network_monitor::NetTracker::new(),
            //active_downloads: Vec::new(),
        };
        app
    }

    pub fn on_build_downloader_button_clicked(&mut self) -> Result<()> {
        self.downloader = Some(
            Arc::new(
                Downloader::default(
                    self.fnames_with_urls.clone().unwrap(),
                    self.url_columns.len(),
                    self.output_folder.clone().unwrap()
                )?
            )
        );
        Ok(())
    }

    pub fn should_show_read_xlsx_button(&self) -> bool {
        if self.xlsx_file == None { return  false; }
        if self.file_name_column == None { return false; }
        if self.url_columns.len() == 0 { return false; }
        true
    }

    pub fn on_read_xlsx_button_clicked(&mut self) -> Result<()> {
        if let Some(path) = &self.xlsx_file {
            self.fnames_with_urls = Some(
                read_xlsx(
                    path, 
                    self.file_name_column.clone().unwrap(), 
                    self.url_columns.clone(),
                )?
            )
        }
        Ok(())    
    }

    pub fn on_filename_clicked(&mut self) {
        self.file_name_column = None;
    }

    pub fn on_url_column_clicked(&mut self, url_index: usize) {
        self.url_columns.remove(url_index);
    }

    pub fn on_download_all_button_clicked(&mut self) -> Result<()> {
        if let Some(dl) = &self.downloader {
            
            self.net_tracker.start_tracking();
            download(dl.clone());

        }

        Ok(())
    }

    fn buttons(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if ui.button("Open file").clicked() {
                        
            self.ui.choose_xlsx.select_file();
        }   
        if let Some(_) = self.ui.choose_xlsx.update(ctx).selected() {
            self.xlsx_file = self.ui.choose_xlsx.take_selected();
            if let Some(path) = &self.xlsx_file {
                self.all_xlsx_column_names = Some(get_xlsx_column_names(&path).unwrap());
            }
            
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

        if let Some(column_names) = &self.all_xlsx_column_names.clone() {
            ui.horizontal(|ui| {

                ui.vertical(|ui| {
                    ui.menu_button("Filename column", |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for cn in &column_names.clone() {
                                if !self.url_columns.contains(&cn.to_string()) {
                                    if ui.button(&cn.clone()).clicked() {
                                        self.file_name_column = Some(cn.clone());
                                        ui.close_menu();
                                    }    
                                }
                            }
                        })
                    });
                    if let Some(file_name_column) = &self.file_name_column {
                        if ui.button(file_name_column).clicked() {
                            self.on_filename_clicked();
                        }
                    }
                });
                ui.vertical(|ui| {
                    ui.menu_button("Url columns", |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for cn in &column_names.clone() {
                                if !&self.url_columns.contains(&cn.to_string()) {
                                    if ui.button(&cn.clone()).clicked() {
                                        self.url_columns.push(cn.clone());
                                        ui.close_menu();
                                    }    
                                }
                            }
                        })
                    });
                    for (i, col) in self.url_columns.clone().iter().enumerate().by_ref() {
                        if ui.button(col).clicked() {
                            self.on_url_column_clicked(i);
                        }
                    } 
                });                    
            });
        }
        if ui.add_enabled(
            self.should_show_read_xlsx_button(), 
            egui::Button::new("Read xlsx")
        ).clicked() {
            self.on_read_xlsx_button_clicked();
        }
        if self.fnames_with_urls.is_some() {
            ui.label("xlsx read!");
        }

        
        ui.horizontal(|ui| {
            if ui.add_enabled(
                self.fnames_with_urls.is_some() && self.output_folder.is_some(),
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
    
    }
    
}


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

fn menu_bar(ui: &mut egui::Ui, ctx: &egui::Context) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ui.add_space(16.0);
        egui::widgets::global_dark_light_mode_buttons(ui);                
    });
}

fn failed_download_results(ui: &mut egui::Ui) {
    
}

fn download_progress(ui: &mut egui::Ui, downloader: &Option<Arc<Downloader>>) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        if let Some(downloader) = downloader {
            if let std_Ok(p) = downloader.progress.lock() {
                for (fname, dp) in p.iter() {
                    if !dp.finished {
                        if let Some(size) = dp.size { 
                            ui.label(format!("{}", fname));
                            ui.add(
                                egui::ProgressBar::new(dp.downloaded as f32 / size as f32)
                                    .show_percentage()
                                    //.text(format!("{}/{}", dp.downloaded, size))
                                    .fill(Color32::DARK_GRAY)
                            );   
                        }
                    }
                }
            }
        }
    });  
}



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