use color_eyre::eyre::Result;
use std::{error::Error, fs::File, io::{copy, Write}, path};
use downloader::get_urls_from_dataframe;
use reqwest::Client;
use tokio::*;
mod xlsx_reader;
use xlsx_reader::*;
mod downloader;
//use anyhow::Result;

static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let file_path = "C:/Users/KOM/dev/rust/pdf_downloader/data/test_file_short.xlsx";

    let df = read_xlsx(file_path)?;

    let client_builder = Client::builder();
    let client = client_builder
        .user_agent(APP_USER_AGENT)
        .build()?;

    let downloader = downloader::Downloader::new(
        client, 
        df, 
        "C:/Users/KOM/dev/rust/pdf_downloader/data/downloaded_files/"
    );

    downloader.download_all().await?;


    //downloader.download_n(4).await?;

    Ok(())

}
    // // let res = downloader::download_pdf("https://www.rust-lang.org/logos/rust-logo-512x512.png");
    // // let response = res.unwrap();
    // let target = "https://www.abancacorporacionbancaria.com/files/documents/memoria-anual-rsc-2016-es.pdf";
    // let response = reqwest::get(target).await?;



    // // let headers = response.headers().clone();
    // // for h in headers {
    // //     println!("{:?}", h);
    // // }

    // let mut dest = File::create("C:/Users/KOM/dev/rust/pdf_downloader/data/test.pdf")?;
    // let content = response.bytes().await?;
    // dest.write_all(&content)?;

    // Ok(())



    // match &res {
    //     Ok(r) => {
    //         println!("{:?}", r);
    //         let mut headers = r.headers().clone();
    //         for h in headers.drain() {
                
    //             match h.0 {
    //                 Some(hn) => println!("{}: {:?}", hn, h.1),
    //                 None => println!("{:?}", h)
    //             }
    //         }
    //     },
    //     Err(e) => println!("{}", e)
    // }
    // let response = res.unwrap();
    // let url = response.url();
    // let path = url.path();
    

    