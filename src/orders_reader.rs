use std::sync::Arc;

use crate::coffee_maker::CoffeeMaker;
use crate::messages::{
    ErrorOpeningFile, FinishedFile, OpenFile, OpenedFile, ProcessOrder, ReadAnOrder,
};
use crate::order::Order;
use actix::fut::ready;
use actix::{
    Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, Context, Handler, Message,
    ResponseActFuture, WrapFuture,
};
use actix_rt::System;
use async_std::fs::File;
use async_std::io::prelude::BufReadExt;
use async_std::io::BufReader;
use async_std::sync::Mutex;
use log::{debug, error, info};

pub struct OrdersReader {
    file_name: String,
    file: Option<Arc<Mutex<BufReader<File>>>>,
    coffee_maker_addr: Option<Addr<CoffeeMaker>>,
}

#[derive(Debug, PartialEq, Eq)]
enum OrdersReaderState {
    Reading(Order),
    ErrorReading,
    ParserErrorRetry,
    Finished,
}

impl OrdersReader {
    pub fn new(file_name: String) -> OrdersReader {
        OrdersReader {
            file: None,
            file_name,
            coffee_maker_addr: None,
        }
    }

    fn try_to_read_next_line(&self, ctx: &mut Context<OrdersReader>) {
        if let Err(e) = ctx.address().try_send(ReadAnOrder) {
            error!(
                "[READER] Error sending message to read next line {}, stopping...",
                e
            );
            System::current().stop();
        }
    }

    fn send_message<ToCoffeeMakerMessage>(&self, msg: ToCoffeeMakerMessage)
    where
        CoffeeMaker: Handler<ToCoffeeMakerMessage>,
        ToCoffeeMakerMessage: Message + Send + 'static,
        ToCoffeeMakerMessage::Result: Send,
    {
        if let Some(addr) = self.coffee_maker_addr.as_ref() {
            if let Err(e) = addr.try_send(msg) {
                error!(
                    "[READER] Error sending message to coffee maker {}, stopping...",
                    e
                );
                System::current().stop();
            }
            return;
        }
        error!("[READER] Address is not present, stopping...");
        System::current().stop();
    }
}

impl Actor for OrdersReader {
    type Context = Context<Self>;
}

impl Handler<OpenFile> for OrdersReader {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: OpenFile, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[READER] Open file started");
        self.coffee_maker_addr = Some(msg.0);
        Box::pin(
            File::open(self.file_name.clone())
                .into_actor(self)
                .map(handle_opened_file),
        )
    }
}

impl Handler<ReadAnOrder> for OrdersReader {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: ReadAnOrder, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[READER] Received message to read an order from the file");
        match self.file.as_ref() {
            Some(file) => Box::pin(
                read_line_from_file(file.clone())
                    .into_actor(self)
                    .map(send_message_depending_on_result),
            ),
            None => {
                error!("[READER] File should be present, stopping...");
                System::current().stop();
                Box::pin(
                    ready(OrdersReaderState::ErrorReading)
                        .into_actor(self)
                        .map(send_message_depending_on_result),
                )
            }
        }
    }
}

async fn read_line_from_file(file: Arc<Mutex<BufReader<File>>>) -> OrdersReaderState {
    let mut file = file.lock().await;
    let mut line = String::new();
    let result = file.read_line(&mut line).await;
    if let Err(e) = result {
        error!("[READER] Error reading file {:?}", e);
        return OrdersReaderState::ErrorReading;
    }
    let bytes_read = result.unwrap();
    if bytes_read == 0 {
        info!("[READER] Finished reading file");
        return OrdersReaderState::Finished;
    }
    debug!("[READER] Line read from file: {}", line);
    let conversion_result = Order::from_line(&line);
    if let Err(e) = conversion_result {
        error!("[READER] Error parsing order from file {:?}", e);
        return OrdersReaderState::ParserErrorRetry;
    }

    OrdersReaderState::Reading(conversion_result.unwrap())
}

fn handle_opened_file(
    result: Result<File, std::io::Error>,
    me: &mut OrdersReader,
    _ctx: &mut Context<OrdersReader>,
) {
    if result.is_err() {
        error!("[READER] Error opening file: {}", me.file_name);
        me.send_message(ErrorOpeningFile);
        return;
    }
    info!("[READER] Opened file: {}", me.file_name);
    me.file = Some(Arc::new(Mutex::new(BufReader::new(result.unwrap()))));
    me.send_message(OpenedFile);
}

fn send_message_depending_on_result(
    result: OrdersReaderState,
    me: &mut OrdersReader,
    ctx: &mut Context<OrdersReader>,
) {
    match result {
        OrdersReaderState::ParserErrorRetry => {
            me.try_to_read_next_line(ctx);
        }
        OrdersReaderState::Reading(order) => {
            me.send_message(ProcessOrder(order));
        }
        _ => {
            me.send_message(FinishedFile);
            ctx.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::order::ConsumptionType;

    use super::*;

    #[actix_rt::test]
    async fn should_read_a_line_from_the_file_and_return_continue_status_with_order() {
        let file = File::open(String::from("tests/one_order.csv")).await;
        if file.is_err() {
            assert!(false);
        }
        let file = file.unwrap();
        let file = Arc::new(Mutex::new(BufReader::new(file)));
        let result = read_line_from_file(file).await;
        match result {
            OrdersReaderState::Reading(order) => assert_eq!(
                Order {
                    consumption_type: ConsumptionType::Cash,
                    consumption: 500,
                    account_id: 1,
                },
                order
            ),
            _ => assert!(false),
        }
    }

    #[actix_rt::test]
    async fn should_return_finished_reading_file() {
        let file = File::open(String::from("tests/empty_file.csv")).await;
        if file.is_err() {
            assert!(false);
        }
        let file = file.unwrap();
        let file = Arc::new(Mutex::new(BufReader::new(file)));
        let result = read_line_from_file(file).await;
        assert_eq!(OrdersReaderState::Finished, result);
    }

    #[actix_rt::test]
    async fn should_return_parser_error_if_the_file_format_is_wrong() {
        let file = File::open(String::from("tests/wrong_format.csv")).await;
        if file.is_err() {
            assert!(false);
        }
        let file = file.unwrap();
        let file = Arc::new(Mutex::new(BufReader::new(file)));
        let result = read_line_from_file(file.clone()).await;
        assert_eq!(OrdersReaderState::ParserErrorRetry, result);

        let result = read_line_from_file(file.clone()).await;
        assert_eq!(OrdersReaderState::ParserErrorRetry, result);

        let result = read_line_from_file(file.clone()).await;
        assert_eq!(OrdersReaderState::ParserErrorRetry, result);

        let result = read_line_from_file(file).await;
        assert_eq!(OrdersReaderState::Finished, result);
    }
}
