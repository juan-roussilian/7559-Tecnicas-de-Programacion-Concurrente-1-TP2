use std::collections::HashMap;
use std::sync::Arc;

use crate::actor_messages::{OpenFile, OpenedFile, ProcessOrder, ReadAnOrder};
use crate::order::Order;
use crate::CoffeeMaker;
use actix::fut::{ready, wrap_future};
use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, ContextFutureSpawner, Handler, Message,
    ResponseActFuture, WrapFuture,
};
use actix_rt::System;
use async_std::fs::File;
use async_std::io::prelude::BufReadExt;
use async_std::io::BufReader;
use async_std::sync::Mutex;
use log::{debug, error, info};

/// Lector de ordenes de la cafetera
pub struct OrdersReader {
    file_name: String,
    file: Option<Arc<Mutex<BufReader<File>>>>,
    coffee_maker_addr: Option<HashMap<usize, Addr<CoffeeMaker>>>,
}

/// Estados posibles al leer una linea del archio
#[derive(Debug, PartialEq, Eq)]
enum OrdersReaderState {
    Reading(Order, usize),
    ErrorReading,
    ParserErrorRetry(usize),
    Finished(usize),
}

impl OrdersReader {
    pub fn new(file_name: String) -> OrdersReader {
        OrdersReader {
            file: None,
            file_name,
            coffee_maker_addr: None,
        }
    }

    fn try_to_read_next_line(&self, ctx: &mut Context<OrdersReader>, id: usize) {
        if let Err(e) = ctx.address().try_send(ReadAnOrder(id)) {
            error!(
                "[READER] Error sending message to read next line {}, stopping...",
                e
            );
            System::current().stop();
        }
    }

    fn finish_file(&mut self, id: usize) {
        if let Some(addresses) = self.coffee_maker_addr.as_mut() {
            addresses.remove(&id);
            if addresses.is_empty() {
                info!("[READER] Everyone finished, stopping...");
                System::current().stop();
            }
        }
    }

    fn send_message<ToCoffeeMakerMessage>(&self, msg: ToCoffeeMakerMessage, id: usize)
    where
        CoffeeMaker: Handler<ToCoffeeMakerMessage>,
        ToCoffeeMakerMessage: Message + Send + 'static,
        ToCoffeeMakerMessage::Result: Send,
    {
        if let Some(addresses) = self.coffee_maker_addr.as_ref() {
            let addr = addresses.get(&id);
            match addr {
                Some(addr) => {
                    if let Err(e) = addr.try_send(msg) {
                        error!(
                            "[READER] Error sending message to coffee maker {}, stopping...",
                            e
                        );
                        System::current().stop();
                    }
                }
                None => {
                    error!("[READER] Address is not present, stopping...");
                    System::current().stop();
                }
            }
            return;
        }
        error!("[READER] Addresses are not present, stopping...");
        System::current().stop();
    }

    fn send_all<ToCoffeeMakerMessage>(&self, msg: ToCoffeeMakerMessage)
    where
        CoffeeMaker: Handler<ToCoffeeMakerMessage>,
        ToCoffeeMakerMessage: Message + Send + 'static + Clone,
        ToCoffeeMakerMessage::Result: Send,
    {
        if let Some(addresses) = self.coffee_maker_addr.as_ref() {
            for id in 0..addresses.len() {
                self.send_message(msg.clone(), id);
            }
            return;
        }
        error!("[READER] Addresses are not present, stopping...");
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
    type Result = ();

    fn handle(&mut self, msg: ReadAnOrder, ctx: &mut Context<Self>) -> Self::Result {
        debug!("[READER] Received message to read an order from the file");
        match self.file.as_ref() {
            Some(file) => wrap_future::<_, Self>(read_line_from_file(file.clone(), msg.0))
                .map(send_message_depending_on_result)
                .spawn(ctx),

            None => {
                error!("[READER] File should be present, stopping...");
                wrap_future::<_, Self>(ready(OrdersReaderState::ErrorReading))
                    .map(send_message_depending_on_result)
                    .spawn(ctx)
            }
        }
    }
}
/// Lee una linea del archivo de pedidos, y retorna un estado dependiendo si esta es la ultima o no,
/// o si fallo la lectura.
async fn read_line_from_file(
    file: Arc<Mutex<BufReader<File>>>,
    coffee_id: usize,
) -> OrdersReaderState {
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
        return OrdersReaderState::Finished(coffee_id);
    }
    debug!("[READER] Line read from file: {}", line);
    let conversion_result = Order::from_line(&line);
    if let Err(e) = conversion_result {
        error!("[READER] Error parsing order from file {:?}", e);
        return OrdersReaderState::ParserErrorRetry(coffee_id);
    }

    OrdersReaderState::Reading(conversion_result.unwrap(), coffee_id)
}
/// Guardara el archivo de pedidos en caso de que este haya sido abierto sin errores y se lo comunica a los otros actores
fn handle_opened_file(
    result: Result<File, std::io::Error>,
    me: &mut OrdersReader,
    _ctx: &mut Context<OrdersReader>,
) {
    if result.is_err() {
        error!("[READER] Error opening file: {}", me.file_name);
        System::current().stop();
        return;
    }
    info!("[READER] Opened file: {}", me.file_name);
    me.file = Some(Arc::new(Mutex::new(BufReader::new(result.unwrap()))));
    me.send_all(OpenedFile);
}
/// Enviara un mensaje a los dependiendo del estado del parser, el cual puede ser: error, leyendo/parseando y terminado.
fn send_message_depending_on_result(
    result: OrdersReaderState,
    me: &mut OrdersReader,
    ctx: &mut Context<OrdersReader>,
) {
    match result {
        OrdersReaderState::ParserErrorRetry(id) => {
            me.try_to_read_next_line(ctx, id);
        }
        OrdersReaderState::Reading(order, id) => {
            me.send_message(ProcessOrder(order), id);
        }
        OrdersReaderState::Finished(id) => {
            me.finish_file(id);
        }
        _ => {}
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
        let result = read_line_from_file(file, 0).await;
        match result {
            OrdersReaderState::Reading(order, 0) => assert_eq!(
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
        let result = read_line_from_file(file, 0).await;
        assert_eq!(OrdersReaderState::Finished(0), result);
    }

    #[actix_rt::test]
    async fn should_return_parser_error_if_the_file_format_is_wrong() {
        let file = File::open(String::from("tests/wrong_format.csv")).await;
        if file.is_err() {
            assert!(false);
        }
        let file = file.unwrap();
        let file = Arc::new(Mutex::new(BufReader::new(file)));
        let result = read_line_from_file(file.clone(), 0).await;
        assert_eq!(OrdersReaderState::ParserErrorRetry(0), result);

        let result = read_line_from_file(file.clone(), 0).await;
        assert_eq!(OrdersReaderState::ParserErrorRetry(0), result);

        let result = read_line_from_file(file.clone(), 0).await;
        assert_eq!(OrdersReaderState::ParserErrorRetry(0), result);

        let result = read_line_from_file(file, 0).await;
        assert_eq!(OrdersReaderState::Finished(0), result);
    }
}
