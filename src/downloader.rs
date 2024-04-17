
use std::{iter::zip, path::PathBuf};

use reqwest::{self, Client, Response, Url, Error};
use polars::{error::PolarsResult, frame::DataFrame};
//use anyhow::{Ok, Result};
//use tokio::fs::write;
use std::result::Result::Ok as std_Ok;
use color_eyre::eyre::*;

#[derive(Debug, Clone)]
pub struct DownloadPDF {
    filename: String,
    urls: AvailableUrls
}

impl DownloadPDF {

    pub fn new(filename: &str, urls: AvailableUrls) -> Self {
        DownloadPDF { 
            filename: filename.to_string(),
            urls, 
        }
    }

    pub async fn download(&self, target_url: Url, client: &Client, destination: &str) -> Result<()>{
        
        let response = client.get(target_url.clone()).send().await?;
        let headers = response.headers();
        let content_type = headers.get("content-type")
            .ok_or_eyre("content-type not received")?;
        if content_type != "application/pdf" {
            return Err(eyre!("{}: content type: {:?}, is not of type application/pdf", self.filename, content_type));
        }
        let content = response.bytes().await?;
        let fd = format!("{}{}{}",destination, self.filename,".pdf");
        tokio::fs::write(fd, &content).await?;        
        Ok(())
    }

    // pub async fn download(&self, downloader: &Downloader, target_url: Url) -> Result<()>{
        
    //     downloader.download_pdf(self.filename.clone(), target_url).await?;
    //     Ok(())
    // }
}

#[derive(Debug, Clone)]
pub enum AvailableUrls {
    Primary{ url: Url },
    Alt{ url: Url },
    Both{ primary: Url, alt: Url },
    //Neither    
}

pub struct Downloader {
    client: Client,
    dataframe: DataFrame,
    file_destination: String
    //file_destination: PathBuf
}

impl Downloader {
    
    pub fn new(client: Client, dataframe: DataFrame, file_destination: &str) -> Self {
        Downloader {
            client,
            dataframe,
            file_destination: file_destination.to_string()
            //file_destination: PathBuf::from(file_destination)
        }
    }

    pub async fn download_pdf(&self, filename: String, target_url: Url) -> Result<()>{
        
        let response = self.client.get(target_url.clone()).send().await?;
        let headers = response.headers();
        let content_type = headers.get("content-type")
            .ok_or_eyre("content-type not received")?;
        if content_type != "application/pdf" {
            return Err(eyre!("{}: content type: {:?}, is not of type application/pdf", filename, content_type));
        }
        let content = response.bytes().await?;
        let fd = format!("{}{}{}",self.file_destination,filename,".pdf");
        tokio::fs::write(fd, &content).await?;        
        Ok(())
    }

    // pub async fn download_pdf(&self, filename: String, target_url: Url) -> Result<()>{
        
    //     let response = self.client.get(target_url).send().await?;
    //     let headers = response.headers();
    //     let content_type = headers.get("content-type")
    //         .ok_or_eyre("content-type not received")?;
    //     if content_type != "application/pdf" {
    //         return Err(eyre!("{}: content type: {:?}, is not of type application/pdf", filename, content_type));
    //     }
    //     let content = response.bytes().await?;
    //     let fd = format!("{}{}{}",self.file_destination,filename,".pdf");
    //     tokio::fs::write(fd, &content).await?;        
    //     Ok(())
    // }


    pub async fn download_all(&self) -> Result<()> {
        let filenames = get_filenames_from_dataframe(&self.dataframe, "BRnum")?;
        let urls = get_urls_from_dataframe(&self.dataframe, "Pdf_URL")?;
        let alt_urls = get_alt_urls_from_dataframe(&self.dataframe, "Report Html Address")?;

        let downloads: Vec<_> = zip(zip(filenames, urls), alt_urls)
            .into_iter()
            .map(
                |((fname, url), alt_url)|{
                    (
                        fname, 
                        validate_url(url), 
                        validate_url(alt_url)
                    )
                })
            .filter_map(|(fname, url, alt_url)| {
                match (fname, url, alt_url) {
                    (Some(f), std_Ok(u), std_Ok(a)) => Some(
                        DownloadPDF::new(f, AvailableUrls::Both { primary: u, alt: a })
                    ),
                    (Some(f), std_Ok(u), Err(_)) => Some(
                        DownloadPDF::new(f, AvailableUrls::Primary { url: u })
                    ),
                    (Some(f), Err(_), std_Ok(a)) => Some(
                        DownloadPDF::new(f, AvailableUrls::Alt { url: a })
                    ),
                    

                    // (Some(f), Some(u), Some(a)) => {
                    //     let parsed_url = Url::parse(u);
                    //     match parsed_url {
                    //         std_Ok(pu) => Some((f, pu)),
                    //         Err(e) => None
                    //     }
                    // },
                    _ => None
                }
            })
            .map(|d| {
                match d.urls {
                    AvailableUrls::Both{ref primary, ref alt} => (d.clone(), self.download_pdf(d.clone().filename, primary.clone())),
                    AvailableUrls::Primary{ref url} => (d.clone(), self.download_pdf(d.filename.clone(), url.clone())),
                    AvailableUrls::Alt{ref url} => (d.clone(), self.download_pdf(d.filename.clone(), url.clone()))
                }
            })
            .collect();
        for (d, f) in downloads {
            match f.await {
                std_Ok(_) => println!("{}, primary url successful", &d.filename),
                Err(e) => match d.urls {
                    AvailableUrls::Both { primary, alt } => {
                        let alt_d = self.download_pdf(d.filename.clone(), alt.clone()).await;
                        let _ = match alt_d {
                            std_Ok(_) => println!("{}, alt url successful", &d.filename),
                            Err(e) => println!("{} unsuccessful: {}", &d.filename, e)
                        };
                    },
                    _ => ()
                }
            } 
        }
        Ok(())
    }
}

pub fn get_urls_from_dataframe<'a>(df: &'a DataFrame, url_col_name: &str) -> Result<Vec<Option<&'a str>>> {
    let ca = df.column(url_col_name)?.str()?;
    let to_vec: Vec<Option<&str>> = ca.into_iter().collect();
    Ok(to_vec)
}

pub fn get_alt_urls_from_dataframe<'a>(df: &'a DataFrame, url_col_name: &str) -> Result<Vec<Option<&'a str>>> {
    let ca = df.column(url_col_name)?.str()?;
    let to_vec: Vec<Option<&str>> = ca.into_iter().collect();
    Ok(to_vec)
}

pub fn get_filenames_from_dataframe<'a>(df: &'a DataFrame, fname_col_name: &str) -> Result<Vec<Option<&'a str>>> {
    let ca = df.column(fname_col_name)?.str()?;
    let to_vec: Vec<Option<&str>> = ca.into_iter().collect();
    Ok(to_vec)
}

pub fn validate_url(url: Option<&str>) -> Result<Url> {
    let r = match url {
        Some(u) => Url::parse(u)?,
        None => return Err( eyre!("No url entry in xlsx-file"))
    };
    Ok(r)
}