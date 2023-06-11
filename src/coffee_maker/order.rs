use log::error;

use crate::errors::CoffeeMakerError;

/// Representa a una orden leida del archivo, tiene el tipo de orden, la cuenta, y los puntos que da o quita
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Order {
    pub consumption_type: ConsumptionType,
    pub account_id: usize,
    pub consumption: usize,
}

/// Los tipos de pedidos que puede haber
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ConsumptionType {
    Points,
    Cash,
}

impl Order {
    pub fn from_line(line: &str) -> Result<Order, CoffeeMakerError> {
        let line = remove_ending(line);
        let parts: Vec<&str> = line.splitn(3, ',').collect();
        if parts.len() != 3 {
            error!("[READER] Check format ',' in file");
            return Err(CoffeeMakerError::FileReaderFormatError);
        }
        let consumption_type = get_consumption_type(parts[0])?;
        let consumption = parts[1].parse::<usize>()?;
        let account_id = parts[2].parse::<usize>()?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_correct_order_and_assert(
        line: &str,
        consumption_type: ConsumptionType,
        consumption: usize,
        account_id: usize,
    ) {
        let result = Order::from_line(line);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(
            Order {
                consumption_type,
                consumption,
                account_id,
            },
            result
        );
    }

    #[test]
    fn should_parse_a_correct_order() {
        parse_correct_order_and_assert("POINTS,5000,123456", ConsumptionType::Points, 5000, 123456);
    }

    #[test]
    fn should_parse_a_correct_order_if_it_has_trailing_characters() {
        parse_correct_order_and_assert("POINTS,5000,123\r\n", ConsumptionType::Points, 5000, 123);
        parse_correct_order_and_assert("CASH,9999,4\r", ConsumptionType::Cash, 9999, 4);
        parse_correct_order_and_assert("CASH,123,4\n", ConsumptionType::Cash, 123, 4);
    }

    #[test]
    fn should_return_format_error() {
        let result = Order::from_line("POINTS,5000,123,23");
        assert!(result.is_err());
        assert_eq!(CoffeeMakerError::FileReaderFormatError, result.unwrap_err());

        let result = Order::from_line("POINTS,5000\r\n");
        assert!(result.is_err());
        assert_eq!(CoffeeMakerError::FileReaderFormatError, result.unwrap_err());

        let result = Order::from_line(",,");
        assert!(result.is_err());
        assert_eq!(CoffeeMakerError::FileReaderFormatError, result.unwrap_err());

        let result = Order::from_line("card,500,500");
        assert!(result.is_err());
        assert_eq!(CoffeeMakerError::FileReaderFormatError, result.unwrap_err());

        let result = Order::from_line("CASH,asd,500");
        assert!(result.is_err());
        assert_eq!(CoffeeMakerError::FileReaderFormatError, result.unwrap_err());

        let result = Order::from_line("CASH,500,asd");
        assert!(result.is_err());
        assert_eq!(CoffeeMakerError::FileReaderFormatError, result.unwrap_err());
    }
}
