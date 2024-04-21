use chrono::{DateTime, Utc};
use std::fmt::Debug;
use std::fs::File;
use std::io::Write;
use std::{any::Any, collections::HashMap, fs, iter::zip, path::PathBuf};
use std::time::*;
use polars::datatypes::{ArrayCollectIterExt, Field};
use reqwest::{self, Client, Response, Url, Error};
use polars::{error::PolarsResult, frame::DataFrame};
use tokio::io::AsyncWriteExt;
//use anyhow::{Ok, Result};
use tokio::fs::write;
use tokio::task::{JoinError, JoinHandle};
use std::result::Result::Ok as std_Ok;
use color_eyre::eyre::*;
use serde;
use crate::xlsx_reader::*;
use futures_util::{Future, StreamExt, TryFutureExt};

use itertools::{self, multizip, Itertools};
use tokio::runtime::Builder;
use std::sync::{Arc, Mutex};


type UrlDb = Arc<Mutex<HashMap<String, (Option<String>, Option<String>)>>>;
type ProgressDb = Arc<Mutex<HashMap<String, DownloadProgress>>>;

static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);

#[derive(Debug)]
pub struct Downloader {
    client: Client,
    //dataframe: DataFrame,
    fnames_with_urls: HashMap<String, (Option<String>, Option<String>)>,
    pub destination_folder: PathBuf,
    pub progress: ProgressDb,
    log_writer: Option<csv::Writer<File>>,
    pub status: AppStatus,
}



#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub size: u64,
    pub finished: bool,
}

impl DownloadProgress {
    
    pub fn new(size: u64) -> Self {
        Self { downloaded: 0, size: size, finished: false }
    }

    pub fn update(&mut self, amount: u64) -> Self {
        self.downloaded += amount;
        if self.downloaded >= self.size {
            self.finished = true;
        }
        self.clone()
    }
}

impl Downloader {
    
    pub fn new(client: Client, fnames_with_urls: HashMap<String, (Option<String>, Option<String>)>, destination_folder: PathBuf) -> Self {
        Downloader {
            client,
            //dataframe,
            fnames_with_urls,
            destination_folder,
            progress: Arc::new(Mutex::new(HashMap::<String, DownloadProgress>::new())),
            log_writer: None,
            status: AppStatus::NotStarted,
            //file_destination: PathBuf::from(file_destination)
        }
    }

    pub fn download_all(&mut self) -> Result<()> {
        self.status = AppStatus::Downloading;
        let mut log_writer = build_log_writer(&self.destination_folder, "download_log".to_string())?;
        let client = self.client.clone();
        let destination_folder = self.destination_folder.clone();
        let progress = self.progress.clone();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap();

        let mut dl_tasks = Vec::new();
        for (fname, urls) in self.fnames_with_urls.iter() {

            let validated_url = validate_url(urls.0.clone());
            
            match validated_url {
                std_Ok(u) => {
                    dl_tasks.push(
                        (
                            fname.clone(),
                            urls.1.clone(),
                            runtime.spawn(
                                download_pdf(
                                    client.clone(),
                                    fname.clone(),
                                    u.clone(),
                                    destination_folder.clone(),
                                    progress.clone(),
                                )
                            )
                        )
                    );
                }
                Err(_) => ()
            }
        }
        let thread = std::thread::spawn(
            move || {
                let mut alt_url_downloads = Vec::new();
                for (fname, alt_url, task) in dl_tasks {
                    let block_res = runtime.block_on(task);
                    let success = match block_res {
                        std_Ok(dl_res) => { 
                            match dl_res {
                                std_Ok(_) => {
                                    let _ = log_writer.write_record([fname.clone(), "true".to_string(), "primary url".to_string()]);
                                    true
                                },
                                Err(_) => {
                                    //log_writer.write_record([fname.clone(), "false".to_string(), "primary url".to_string()]);
                                    false
                                },
                            }
                        },
                        Err(_) => {
                            //log_writer.write_record([fname.clone(), "false".to_string(), "primary url".to_string()]);
                            false
                        }
                    };
                    if !success {
                        alt_url_downloads.push((fname,alt_url));
                    }
                }
                //let _ = log_writer.flush();

                let mut alt_dl_tasks = Vec::new();
                for (fname, url) in alt_url_downloads {
                    match validate_url(url.clone()) {
                        std_Ok(url) => {
                            alt_dl_tasks.push(
                                (
                                    fname.clone(),
                                    runtime.spawn(
                                        download_pdf(
                                            client.clone(),
                                            fname.clone(),
                                            url.clone(),
                                            destination_folder.clone(),
                                            progress.clone(),
                                        )
                                    )
                                )
                            );
                        },
                        Err(_) => {
                            let _ = log_writer.write_record([fname.clone(), "false".to_string(), "none".to_string()]);
                        }
                    }
                }
                for (fname, task) in alt_dl_tasks {
                    let alt_block_res = runtime.block_on(task);
                    match alt_block_res {
                        std_Ok(r) => {
                            match r {
                                std_Ok(_) => {
                                    let _ = log_writer.write_record([fname.clone(), "true".to_string(), "alt url".to_string()]);
                                }
                                Err(_) => {
                                    let _ = log_writer.write_record([fname.clone(), "false".to_string(), "none".to_string()]);
                                }
                            }
                        },
                        Err(_) => {
                            let _ = log_writer.write_record([fname.clone(), "false".to_string(), "none".to_string()]);
                        }
                    }
                }
                
                match log_writer.flush() {
                    Err(e) => println!("{}", e),
                    std_Ok(_) => println!("flushed ok")
                }

                
                //(alt_url_downloads, log_writer, runtime, client)
            }
            
        );       
        // let (alt_url_downloads, mut log_writer, runtime, client) = thread.join().unwrap();
        // let mut alt_dl_tasks = Vec::new();
        // for (fname, url) in alt_url_downloads {
        //     match validate_url(url.clone()) {
        //         std_Ok(url) => {
        //             alt_dl_tasks.push(
        //                 (
        //                     fname.clone(),
        //                     runtime.spawn(
        //                         download_pdf(
        //                             client.clone(),
        //                             fname.clone(),
        //                             url.clone(),
        //                             destination_folder.clone(),
        //                             progress.clone(),
        //                         )
        //                     )
        //                 )
        //             );
        //         },
        //         Err(_) => {
        //             let _ = log_writer.write_record([fname.clone(), "false".to_string(), "none".to_string()]);
        //         }
        //     }
        // }


        // let _ = std::thread::spawn(
        //     move || {
        //         for (fname, task) in alt_dl_tasks {
        //             let alt_block_res = runtime.block_on(task);
        //             match alt_block_res {
        //                 std_Ok(_) => {
        //                     let _ =log_writer.write_record([fname.clone(), "true".to_string(), "alt url".to_string()]);
        //                 }
        //                 Err(_) => {
        //                     let _ = log_writer.write_record([fname.clone(), "false".to_string(), "none".to_string()]);
        //                 }
        //             }
        //         }
        //         let _ = log_writer.flush();    
        //     }
        // );
        self.status = AppStatus::Finished;
        Ok(())  
        
    }

    // pub fn set_log_writer(&mut self) -> Result<()> {
    //     self.log_writer = Some(
    //         build_log_writer(&self.destination_folder, "download_log".to_string())?
    //     );
    //     Ok(())
    // }


}

pub async fn download_pdf(
    client: Client, 
    filename: String, 
    url: Url, 
    destination_folder: PathBuf, 
    progress_db: ProgressDb
) -> Result<String> {

    let response = client.get(url.clone()).send().await?;
    let headers = response.headers();
    
    // Check content-type
    let content_type = headers.get("content-type")
        .ok_or_eyre("content-type not received")?;
    if content_type != "application/pdf" {
        return Err(eyre!("{}: content type: {:?}, 
            is not of type application/pdf", filename, content_type));
    }

    // Get content size
    let file_size = response.content_length()
        .ok_or_eyre("No content length of file")?;

    // Create output pdf file
    let mut output_file = destination_folder.clone();
    output_file.push(&filename);
    output_file.set_extension("pdf");
    let mut file = tokio::fs::File::create(output_file).await?;

    // Initialize storage of the download progress.
    {
        let mut p_db = progress_db.lock().unwrap();
        if let None = p_db.get(&filename) {
            let dp = DownloadProgress::new(file_size);
                    p_db.insert(
                        filename.clone(), 
                        dp
                    );
        }
    }

    // Get content as stream
    let mut content = response.bytes_stream();

    // Save incoming stream chunks to file and update download progress    
    while let Some(stream_chunk) = content.next().await {
        let chunk = stream_chunk?;
        file.write_all(&chunk).await?;
        let mut p_db = progress_db.lock().unwrap();
        if let Some(dp) = p_db.get(&filename) {
            let p = dp.clone().update(chunk.len() as u64);
                p_db.insert(
                    filename.clone(), 
                    p
                );
        }
    }

    Ok(filename)
}

pub fn build_downloader(dataframe: DataFrame, destination_folder: PathBuf, filename_col: String, url_col_names: Vec<String>) -> Result<Downloader> {
    let client_builder = Client::builder();
    let client = client_builder
        .user_agent(APP_USER_AGENT)
        .build()?;
    
    let file_names = get_filenames_from_dataframe(&dataframe, &filename_col)?;
    let url_vecs = url_col_names.into_iter().filter_map(
        |col_name| {
            match get_urls_from_dataframe(&dataframe, &col_name) {
                std_Ok(urls) => Some(urls),
                Err(e) => None,
            }
        }
    ).collect_vec();

    let fnames_with_urls: HashMap<String, (Option<String>, Option<String>)> = match url_vecs.len() {
        0 => return Err(eyre!("No urls found")),
        1 => {
            multizip((file_names, url_vecs[0].clone()))
            .filter_map(
                |(a, b)| match a {
                    Some(fname) => Some((fname, (b, None))),
                    None => None,
                }
            ).collect()
        },
        _ => {
            multizip((file_names, url_vecs[0].clone() , url_vecs[1].clone()))
            .filter_map(
                |(a, b, c)| match a {
                    Some(fname) => Some((fname, (b, c))),
                    None => None,
                }
            ).collect()
        },
    };
    
    let downloader = Downloader::new(
        client, 
        fnames_with_urls,
        destination_folder,
    );
    Ok(downloader)
}

pub fn get_urls_from_dataframe<'a>(df: &'a DataFrame, url_col_name: &str) -> Result<Vec<Option<String>>> {
    let ca = df.column(url_col_name)?.str()?;
    let to_vec: Vec<Option<String>> = ca.into_iter().map(
        |s| match s {
            Some(ss) => Some(ss.to_owned()),
            None => None,
        }
    ).collect();
    Ok(to_vec)
}

// pub fn get_alt_urls_from_dataframe<'a>(df: &'a DataFrame, url_col_name: &str) -> Result<Vec<Option<&'a str>>> {
//     let ca = df.column(url_col_name)?.str()?;
//     let to_vec: Vec<Option<&str>> = ca.into_iter().collect();
//     Ok(to_vec)
// }

pub fn get_filenames_from_dataframe(df: &DataFrame, fname_col_name: &str) -> Result<Vec<Option<String>>> {
    let ca = df.column(fname_col_name)?.str()?;
    let to_vec: Vec<Option<String>> = ca.into_iter().map(
        |s| match s {
            Some(ss) => Some(ss.to_owned()),
            None => None,
        }
    ).collect();
    Ok(to_vec)
}

pub fn validate_url(url: Option<String>) -> Result<Url> {
    let r = match url {
        Some(u) => Url::parse(&u)?,
        None => return Err( eyre!("No url entry in xlsx-file"))
    };
    
    Ok(r)
}

pub fn build_log_writer(folder: &PathBuf, filename: String) -> Result<csv::Writer<File>> {
    //let time = chrono::Local::now().to_string();
    let mut filepath = folder.clone();
    filepath.push(&filename);
    // filepath.push("_");
    // filepath.push(&time);
    filepath.set_extension("csv");
    // let mut log_file = std::fs::OpenOptions::new()
    // .create_new(true)
    // .append(true)
    // .open(filepath)?;
    let writer = csv::Writer::from_path(filepath)?;
    Ok(writer)
}

#[derive(Debug)]
pub enum AppStatus {
    NotStarted,
    Downloading,
    FlushingLog,
    Finished,

}
    // pub async fn download_all(&self) -> Result<()> {
    //     let filenames = get_filenames_from_dataframe(&self.dataframe, "BRnum")?;
    //     let urls = get_urls_from_dataframe(&self.dataframe, "Pdf_URL")?;
    //     let alt_urls = get_alt_urls_from_dataframe(&self.dataframe, "Report Html Address")?;

    //     let downloads: Vec<_> = zip(zip(filenames, urls), alt_urls)
    //         .into_iter()
    //         .map(
    //             |((fname, url), alt_url)|{
    //                 (
    //                     fname, 
    //                     validate_url(url), 
    //                     validate_url(alt_url)
    //                 )
    //             })
    //         .filter_map(|(fname, url, alt_url)| {
    //             match (fname, url, alt_url) {
    //                 (Some(f), std_Ok(u), std_Ok(a)) => Some(
    //                     DownloadPDF::new(f, AvailableUrls::Both { primary: u, alt: a })
    //                 ),
    //                 (Some(f), std_Ok(u), Err(_)) => Some(
    //                     DownloadPDF::new(f, AvailableUrls::Primary { url: u })
    //                 ),
    //                 (Some(f), Err(_), std_Ok(a)) => Some(
    //                     DownloadPDF::new(f, AvailableUrls::Alt { url: a })
    //                 ),
    //                 _ => None
    //             }
    //         })
    //         .map(|d| {
    //             match d.urls {
    //                 AvailableUrls::Both{ref primary, ref alt} => (d.clone(), self.download_pdf(d.clone().filename, primary.clone())),
    //                 AvailableUrls::Primary{ref url} => (d.clone(), self.download_pdf(d.filename.clone(), url.clone())),
    //                 AvailableUrls::Alt{ref url} => (d.clone(), self.download_pdf(d.filename.clone(), url.clone()))
    //             }
    //         })
    //         .collect();
    //     for (d, f) in downloads {
    //         match f.await {
    //             std_Ok(_) => println!("{}, {:?} url successful", &d.filename, d.urls ),
    //             Err(e) => match d.urls {
    //                 AvailableUrls::Both { primary, alt } => {
    //                     let alt_d = self.download_pdf(d.filename.clone(), alt.clone()).await;
    //                     let _ = match alt_d {
    //                         std_Ok(_) => println!("{}, alt url successful", &d.filename),
    //                         Err(e) => println!("{} unsuccessful: {}", &d.filename, e)
    //                     };
    //                 },
    //                 _ => ()
    //             }
    //         } 
    //     }
    //     Ok(())
    // }

    // let filenames = get_filenames_from_dataframe(&self.dataframe, "BRnum")?;
        // let urls = get_urls_from_dataframe(&self.dataframe, "Pdf_URL")?;
        // let alt_urls = get_alt_urls_from_dataframe(&self.dataframe, "Report Html Address")?;

        // let downloads: Vec<_> = zip(zip(filenames, urls), alt_urls)
        //     .into_iter()
        //     .map(
        //         |((fname, url), alt_url)|{
        //             (
        //                 fname, 
        //                 validate_url(url), 
        //                 validate_url(alt_url)
        //             )
        //         })
        //     .filter_map(|(fname, url, alt_url)| {
        //         match (fname, url, alt_url) {
        //             (Some(f), std_Ok(u), std_Ok(a)) => Some(
        //                 DownloadPDF::new(f, AvailableUrls::Both { primary: u, alt: a })
        //             ),
        //             (Some(f), std_Ok(u), Err(_)) => Some(
        //                 DownloadPDF::new(f, AvailableUrls::Primary { url: u })
        //             ),
        //             (Some(f), Err(_), std_Ok(a)) => Some(
        //                 DownloadPDF::new(f, AvailableUrls::Alt { url: a })
        //             ),
        //             _ => None
        //         }
        //     })
        //     .map(|d| {
        //         match d.urls {
        //             AvailableUrls::Both{ref primary, ref alt} => (d.clone(), self.download_pdf(d.clone().filename, primary.clone())),
        //             AvailableUrls::Primary{ref url} => (d.clone(), self.download_pdf(d.filename.clone(), url.clone())),
        //             AvailableUrls::Alt{ref url} => (d.clone(), self.download_pdf(d.filename.clone(), url.clone()))
        //         }
        //     })
        //     .collect();
        // for (d, f) in downloads {
        //     match f.await {
        //         std_Ok(_) => println!("{}, {:?} url successful", &d.filename, d.urls ),
        //         Err(e) => match d.urls {
        //             AvailableUrls::Both { primary, alt } => {
        //                 let alt_d = self.download_pdf(d.filename.clone(), alt.clone()).await;
        //                 let _ = match alt_d {
        //                     std_Ok(_) => println!("{}, alt url successful", &d.filename),
        //                     Err(e) => println!("{} unsuccessful: {}", &d.filename, e)
        //                 };
        //             },
        //             _ => ()
        //         }
        //     } 
        // }
        
        // impl DownloadPDF {

//     pub fn new(filename: &str, urls: AvailableUrls) -> Self {
//         DownloadPDF { 
//             filename: filename.to_string(),
//             urls, 
//         }
//     }

    // pub async fn download(&self, target_url: Url, client: &Client, destination: &str) -> Result<()>{
        
    //     let response = client.get(target_url.clone()).send().await?;
    //     let headers = response.headers();
    //     let content_type = headers.get("content-type")
    //         .ok_or_eyre("content-type not received")?;
    //     if content_type != "application/pdf" {
    //         return Err(eyre!("{}: content type: {:?}, is not of type application/pdf", self.filename, content_type));
    //     }
    //     let mut body = response.bytes_stream();

    //     //let content = response.bytes().await?;
    //     let fname = format!("{}{}{}",destination, self.filename,".pdf");
    //     while let Some(chunk) = body.next().await {
    //         fs::write(&fname, chunk?)?
    //     }
    //     Ok(())
    // }
// }

