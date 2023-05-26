use log::debug;

use crate::errors::CoffeeMakerError;

#[derive(Debug)]
pub struct Order {
    pub consumption_type: ConsumptionType,
    pub account_id: usize,
    pub consumption: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConsumptionType {
    Points,
    Cash,
}

impl Order {
    pub fn from_line(line: &str) -> Result<Order, CoffeeMakerError> {
        let line = remove_ending(line);
        let parts: Vec<&str> = line.splitn(3, ',').collect();
        if parts.len() != 3 {
            debug!("Check ',' in file");
            return Err(CoffeeMakerError::FileReaderFormatError);
        }
        let consumption_type = get_consumption_type(parts[0])?;
        let account_id = parts[1].parse::<usize>()?;
        let consumption = parts[2].parse::<usize>()?;
        Ok(Order {
            consumption_type,
            account_id,
            consumption,
        })
    }
}

fn remove_ending(line: &str) -> &str {
    if line.ends_with("\r\n") {
        return line.trim_end_matches("\r\n");
    } else if line.ends_with('\n') {
        return line.trim_end_matches('\n');
    } else if line.ends_with('\r') {
        return line.trim_end_matches('\r');
    }
    line
}

fn get_consumption_type(consumption: &str) -> Result<ConsumptionType, CoffeeMakerError> {
    if consumption.eq("POINTS") {
        return Ok(ConsumptionType::Points);
    }
    if consumption.eq("CASH") {
        return Ok(ConsumptionType::Cash);
    }
    Err(CoffeeMakerError::FileReaderFormatError)
}
