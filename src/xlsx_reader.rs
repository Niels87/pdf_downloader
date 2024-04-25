
use calamine::{open_workbook, Data, RangeDeserializerBuilder, Reader, Xlsx};
use std::collections::HashMap;
use eyre::{eyre, Result};
use std::{path::Path, result::Result::Ok as std_Ok};

/// Read data from an .xlsx file to a hashmap, 
/// where keys are the column headers and values are 
/// vectors containing column data.
/// ISSUE: What happens if filename is empty in xlsx?
pub fn read_xlsx(path: &Path, filename_header: String, url_headers: Vec<String>) -> Result<HashMap<String, Vec<String>>> {
    let mut workbook: Xlsx<_> = open_workbook(&path)?;
    
    let mut headers = url_headers;
    headers.push(filename_header);
    
    let Some(range_result) = workbook.worksheet_range_at(0) else {
        return Err(eyre!("Sheet not in workbook"));
    };
    let range = range_result?;

    let xlsx_data: HashMap<String, Vec<String>> = RangeDeserializerBuilder::
        with_headers(
            &headers
        )
        .from_range::<Data, Vec<String>>(&range)?
        .into_iter()
        .filter_map(
            |row| {
                match row {
                    Err(_) => None,
                    std_Ok(mut r) => {
                        match r.pop() {
                            None => None,
                            Some(fname) => Some(( fname, r ))
                        }
                    }
                }
            }
        )
        .collect();
    
    Ok(xlsx_data)
}   

/// Get the first row of the xlsx file.
pub fn get_xlsx_column_names(path: &Path) -> Result<Vec<String>> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;
    let Some(range_result) = workbook.worksheet_range_at(0) else {
        return Err(eyre!("Sheet not in workbook"));
    };
    let range = range_result?;
    let Some(first_row) = range.rows().next() else {
        return Err(eyre!("First row of xlsx could not be read"));
    };
    let mut column_names: Vec<String> = first_row.into_iter().map(
        |c| {
            c.to_string()
        }
    ).collect();
    column_names.sort_unstable();
    Ok(column_names)
}