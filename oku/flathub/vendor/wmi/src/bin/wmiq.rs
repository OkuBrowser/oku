use std::{collections::HashMap, env::args};
use wmi::{Variant, WMIConnection, WMIResult};

fn main() -> WMIResult<()> {
    let wmi_con = WMIConnection::new()?;
    let args: Vec<String> = args().collect();
    let query = match args.get(1) {
        None => {
            println!("Expected an argument with a WMI query");
            return Ok(());
        }
        Some(query) => query,
    };

    let results: Vec<HashMap<String, Variant>> = match wmi_con.raw_query(query) {
        Err(e) => {
            println!("Couldn't run query {} because of {:?}", query, e);
            return Ok(());
        }
        Ok(results) => results,
    };

    for (i, res) in results.iter().enumerate() {
        println!("Result {}", i);
        println!("{:#?}", res);
    }

    Ok(())
}
