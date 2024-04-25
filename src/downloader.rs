use rayon::{prelude::*, result};
use chrono::{DateTime, Utc};
use futures_util::stream::FuturesUnordered;
use tokio::task::JoinError;
use std::fmt::{Debug, UpperHex};
use std::fs::File;
use std::{collections::HashMap,  path::PathBuf};
use reqwest::{self, Client, Response, Url, Error};
use tokio::io::AsyncWriteExt;
use std::result::Result::Ok as std_Ok;
use eyre::{eyre, Ok, OptionExt, Result};
use futures_util::{Future, StreamExt, TryFutureExt};

use std::sync::{Arc, Mutex};

type ProgressDb = Arc<Mutex<HashMap<String, DownloadProgress>>>;

static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);

#[derive(Debug, Clone)]
pub struct Downloader {
    client: Client,
    fnames_with_urls: HashMap<String, Vec<String>>,
    nr_of_urls: usize,
    pub destination_folder: PathBuf,
    pub progress: ProgressDb,
    //log_writer: csv::Writer<File>,
    //pub status: AppStatus,
}

impl Downloader {
    
    pub fn default(fnames_with_urls: HashMap<String, Vec<String>>, nr_of_urls: usize,  destination_folder: PathBuf) -> Result<Self> {
        let client_builder = Client::builder();
        let client = client_builder
        .user_agent(APP_USER_AGENT)
        .build()?;
        
        let downloader = Downloader {
            client,
            fnames_with_urls,
            nr_of_urls,
            destination_folder: destination_folder.clone(),
            progress: Arc::new(Mutex::new(HashMap::<String, DownloadProgress>::new())),
            //log_writer: build_log_writer(destination_folder.clone(), "download_log".to_string())?,
            //status: AppStatus::NotStarted,
        };
        Ok(downloader)
    }
}

pub fn download(downloader: Arc<Downloader>) -> Result<()> {
    
    std::thread::spawn(
        || {
            download_all(downloader, 10)?;
            Ok(())
        }
    );

    
    
    Ok(())
}

    #[tokio::main]
    pub async fn download_all(downloader: Arc<Downloader> , concurrent_downloads: usize) -> Result<()> {

        //self.status = AppStatus::Downloading;
        let mut log_writer = build_log_writer(downloader.destination_folder.clone(), "download_log".to_string())?;
        let client = downloader.client.clone();
        let fnames_with_urls = downloader.fnames_with_urls.clone();
        let destination_folder = downloader.destination_folder.clone();
        let nr_of_urls = downloader.nr_of_urls.clone();
        let progress = downloader.progress.clone();
        // let runtime = tokio::runtime::Builder::new_multi_thread()
        //     .worker_threads(4)
        //     .enable_all()
        //     .build()
        //     .unwrap();
        
        //let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build()?;
        

        let mut download_futures = FuturesUnordered::new();
        let mut download_results = Vec::new();

        for (fname, urls) in fnames_with_urls.iter() {

            download_futures.push(
            download_pdf(
                client.clone(),
                fname.clone(),
                urls[0].clone(),
                destination_folder.clone(),
                progress.clone(),
                )
            );
            if download_futures.len() >= concurrent_downloads {
                if let Some(result) = download_futures.next().await {
                    println!("{:?}", result);
                    download_results.push(result);
                }
                
            }


            // let validated_url = validate_url(urls[0].clone());
            // match validated_url {
            //     std_Ok(u) => {
            //         fut_unord.push(
            //             (
            //                 fname.clone(),
            //                 runtime.spawn(
            //                     download_pdf(
            //                         client.clone(),
            //                         fname.clone(),
            //                         u.clone(),
            //                         destination_folder.clone(),
            //                         progress.clone(),
            //                     )
            //                 )
            //             )
            //         );
            //     }
            //     Err(_) => ()
            // }
        }

        while let Some(result) = download_futures.next().await {
            println!("{:?}", result);
            download_results.push(result);   
        }
        // let _ = std::thread::spawn(
        //     move || {
        //         //let mut succesful_downloads = Vec::new();
        //         for i in 0..=nr_of_urls-1 {
        //             println!("({})----------------------------------", i);
        //             let mut dl_tasks = Vec::new();
        //             for (fname, urls) in fnames_with_urls.iter() {
        //                 let validated_url = validate_url(urls[i].clone());
        //                 match validated_url {
        //                     std_Ok(u) => {
        //                         dl_tasks.push(
        //                             (
        //                                 fname.clone(),
        //                                 runtime.spawn(
        //                                     download_pdf(
        //                                         client.clone(),
        //                                         fname.clone(),
        //                                         u.clone(),
        //                                         destination_folder.clone(),
        //                                         progress.clone(),
        //                                     )
        //                                 )
        //                             )
        //                         );
        //                     }
        //                     Err(_) => ()
        //                 }
        //             }
        //             for (fname, task) in dl_tasks {
        //                 //println!("({}) blocking on {}", i, fname);
        //                 //let block_res = runtime.block_on(task);
        //                 let block_res = tokio::task::spawn_blocking(
        //                     || {task}
        //                 );
        //                 //block_res.await?
        //                 // let success = match block_res {
        //                 //     std_Ok(dl_res) => { 
        //                 //         match dl_res {
        //                 //             std_Ok(r) => {
        //                 //                 let _ = log_writer.write_record([fname.clone(), "true".to_string()]);
        //                 //                 set_download_result(
        //                 //                     &fname, 
        //                 //                     progress.clone(), 
        //                 //                     Ok(r)
        //                 //                 );
        //                 //                 true
        //                 //             },
        //                 //             Err(e) => {
        //                 //                 set_download_result(
        //                 //                     &fname, 
        //                 //                     progress.clone(), 
        //                 //                     Err(e.into())
        //                 //                 );
        //                 //                 false
        //                 //             },
        //                 //         }
        //                 //     },
        //                 //     Err(e) => {
        //                 //         set_download_result(
        //                 //             &fname, 
        //                 //             progress.clone(), 
        //                 //             Err(e.into())
        //                 //         );
        //                 //         false
        //                 //     }
        //                 // };
        //                 // if success {
        //                 //     fnames_with_urls.remove(&fname);
        //                 // }
        //             }
        //         }
        //         for (fname, _) in fnames_with_urls {
        //             let _ = log_writer.write_record([fname.clone(), "false".to_string()]);
        //         }
        //         log_writer.flush()?;
        //         println!("log flushed");
        //         std_Ok::<(), eyre::Error>(())
        //     }
        // );
        Ok(())
    }

pub async fn download_pdf(
    client: Client, 
    filename: String, 
    url: String, 
    destination_folder: PathBuf, 
    progress_db: ProgressDb
) -> Result<String> {
    
    let validated_url = validate_url(url)?;
    //println!("download_pdf, {}", filename);
    println!("{} starting", filename);

    // Initialize storage of the download progress.
    {
        let mut p_db = progress_db.lock().unwrap();
        if let None = p_db.get(&filename) {
            let dp = DownloadProgress::new();
                    p_db.insert(
                        filename.clone(), 
                        dp
                    );
        }
    }
    set_download_status(&filename, progress_db.clone(), "sending request");
    let response = client.get(validated_url.clone()).send().await?;
    //let response = client.get(url.clone()).send().await?;
    let headers = response.headers();
    let header_length = &headers.len();
    let mut header_strings = Vec::new();
    for (key, val) in headers.clone() {
        header_strings.push(format!("{:?}: {:?}", key, val));
    }

    set_download_status(&filename, progress_db.clone(), "checking content type");
    // Check content-type
    let content_type = headers.get("content-type").ok_or_eyre("no content-type")?;
    if content_type != "application/pdf" { 
        return Err(eyre!("content type: {:?}", content_type));
    }
    set_download_status(&filename, progress_db.clone(), "getting content size");
    let size = {
        // Get content size
        let size = response.content_length().ok_or_eyre("No content length of file")?;
        let mut p_db = progress_db.lock().unwrap();
        if let Some(dp) = p_db.get_mut(&filename) {
            dp.set_size(size);
        }
        size
    };
    set_download_status(&filename, progress_db.clone(), "creating output file");
    // Create output pdf file
    let mut output_file = destination_folder.clone();
    output_file.push(&filename);
    output_file.set_extension("pdf");
    let mut file = tokio::fs::File::create(output_file).await?;

    set_download_status(&filename, progress_db.clone(), "setting up byte stream");
    // Get content as stream
    let mut content = response.bytes_stream();


    set_download_status(&filename, progress_db.clone(), "downloading");
    // Save incoming stream chunks to file and update download progress    
    while let Some(stream_chunk) = content.next().await {
        let chunk = stream_chunk?;
        file.write_all(&chunk).await?;
        let mut p_db = progress_db.lock().unwrap();
        if let Some(dp) = p_db.get_mut(&filename) {
            // if dp.downloaded == 0 {
            //     println!("{}", &filename);
            //     for h in &header_strings {
            //         println!("{}", &h);
            //     }
            //     println!("chunk: {:?}", chunk.len());
            //     println!("content-length: {:?}", size);

            //     let c_slice = chunk.slice(0..=4);
            //     println!("{:?} : {:X}", &c_slice, &c_slice);
            // }
            dp.update(chunk.len() as u64);
        }
    }
    set_download_status(&filename, progress_db.clone(), "finished download");
    Ok(filename)
}

#[derive(Debug)]
pub enum AppStatus {
    NotStarted,
    Downloading,
    FlushingLog,
    Finished,

}

pub fn validate_url(url: String) -> Result<Url> {
    let parsed_url = Url::parse(&url)?;
    Ok(parsed_url)
}

pub fn build_log_writer(folder: PathBuf, filename: String) -> Result<csv::Writer<File>> {
    let filename_time_stamped = format!(
        "{}_{}", 
        filename, 
        chrono::Local::now().format("%d_%m_%y_%H_%M")
    );
    let mut filepath = folder.clone();
    filepath.push(&filename_time_stamped);
    filepath.set_extension("csv");
    let log_file = std::fs::OpenOptions::new()
    .create_new(true)
    .append(true)
    .open(filepath)?;
    Ok(csv::Writer::from_writer(log_file))
}

#[derive(Debug)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub size: Option<u64>,
    pub status: String,
    pub result: Option<Result<String>>,
    pub finished: bool,
}

impl DownloadProgress {
    pub fn new() -> Self {
        Self { 
            downloaded: 0, 
            size: None, 
            status: "starting".to_string(),
            result: None,
            finished: false 
        }
    }
    pub fn update(&mut self, amount: u64) {
        if let Some(size) = self.size  {
            self.downloaded += amount;
            if self.downloaded >= size {
                self.finished = true;
                self.status = "finished".to_string();
            }    
        }
    }
    pub fn set_size(&mut self, size: u64) {
        self.size = Some(size);
    }
}

#[derive(Debug, Clone)]
enum ServerResponse {
    RequestNotSend,
    Ok,
    Err{ code: String },

}

fn set_download_status(filename: &str, progress_db: ProgressDb, status: &str) {

    let mut p_db = progress_db.lock().unwrap();
    if let Some(dp) = p_db.get_mut(filename)  {
        dp.status = status.to_string();
    }
}

fn set_download_result(
    filename: &str, 
    progress_db: ProgressDb, 
    result: Result<String>
) {
    let mut p_db = progress_db.lock().unwrap();
    if let Some(dp) = p_db.get_mut(filename)  {
        dp.result = Some(result);
    }
    
}