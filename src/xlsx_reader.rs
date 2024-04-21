
use calamine::{open_workbook, Data, RangeDeserializer, RangeDeserializerBuilder, Reader, Xlsx};
use itertools::Itertools;
use polars::{frame::DataFrame, series::Series};
use color_eyre::eyre::*;
use std::{path::Path, result::Result::Ok as std_Ok};

pub fn read_xlsx(path: &Path) -> Result<DataFrame> {

    let mut workbook: Xlsx<_> = open_workbook(path)?; 
    let headers = ["BRnum", "Pdf_URL","Report Html Address"];
    let Some(range_result) = workbook.worksheet_range_at(0) else {
        return Err(eyre!("Sheet not in workbook"));
    };
    let range = range_result?;
    // for c in range.rows().next().unwrap() {
    //     println!("col: {:?}", c);    
        
    // }
    let columns = headers.into_iter().filter_map(
        |h| {
            let range_deserializer_result: Result<RangeDeserializer<'_, Data, [String;1]>, calamine::DeError> = 
                RangeDeserializerBuilder::with_headers(&[h]).from_range(&range);
            let mut series: Series = match range_deserializer_result {
                std_Ok(rd) => {
                    rd.map(
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
                    ).collect()
                },
                Err(e) => return None,
            };
            series.rename(h);
            Some(series)
        }
    ).collect();
    let df = DataFrame::new(columns).unwrap();
    Ok(df)
}   

pub fn get_xlsx_column_names(path: &Path) -> Result<Vec<String>> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;
    let Some(range_result) = workbook.worksheet_range_at(0) else {
        return Err(eyre!("Sheet not in workbook"));
    };
    let range = range_result?;
    let Some(first_row) = range.rows().next() else {
        return Err(eyre!("First row of xlsx could not be read"));
    };
    let column_names = first_row.into_iter().map(
        |c| {
            c.to_string()
        }
    ).collect_vec();
    Ok(column_names)
}
