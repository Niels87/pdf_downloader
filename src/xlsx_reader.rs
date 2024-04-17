
use calamine::{open_workbook, Data, RangeDeserializer, RangeDeserializerBuilder, Reader, Xlsx};
use polars::{frame::DataFrame, series::Series};
use color_eyre::eyre::*;
use std::result::Result::Ok as std_Ok;

pub fn read_xlsx(path: &str) -> Result<DataFrame> {

    let mut workbook: Xlsx<_> = open_workbook(path)?; //.expect(&format!("Cannot open file at {}", path));
        
    let headers = ["BRnum", "Pdf_URL","Report Html Address"];

    let Some(range_result) = workbook.worksheet_range_at(0) else {
        return Err(eyre!("Sheet not in workbook"));
    };
    let range = range_result?;

    let columns = headers.into_iter().map(
        |h| {
            let ra_iter: Result<RangeDeserializer<'_, Data, [String;1]>, calamine::DeError> 
                = RangeDeserializerBuilder::with_headers(&[h]).from_range(&range);
            // match &ra_iter {
            //     Ok(i) => println!("ra_iter OK"),
            //     Err(e) => println!("{}", e)
            // }
            let mut unwrapped_ra_iter: Series = ra_iter.unwrap().map(
                |ri| {
                    match ri {
                        std_Ok(s) => { 
                            s[0].clone() 
                        },
                        Err(e) => { 
                            "".to_string() 
                        }
                    }
                }
            ).collect();
            unwrapped_ra_iter.rename(h);
            //println!("{:?}", unwrapped_ra_iter.dtype() );
            unwrapped_ra_iter
        }
    ).collect();
    let df = DataFrame::new(columns).unwrap();
    Ok(df)
}   

