use actix::{
    Actor, ActorContext, ActorFutureExt, Addr, Context, Handler, Message, ResponseActFuture,
    WrapFuture,
};
use actix_rt::System;
use log::{debug, error, info};
use std::env;

use crate::errors::ServerError;
use crate::messages::{
    ErrorOpeningFile, FinishedFile, OpenFile, OpenedFile, ProcessOrder, ReadAnOrder,
};
use crate::order::ConsumptionType;
use crate::orders_reader::OrdersReader;
use crate::randomizer::{Randomizer, RealRandomizer};
use crate::server::{LocalServer, Server};

pub struct CoffeeMaker {
    reader_addr: Addr<OrdersReader>,
    server_conn: Box<dyn Server>,
    order_randomizer: Box<dyn Randomizer>,
}

impl CoffeeMaker {
    fn send_message<ToReaderMessage>(&self, msg: ToReaderMessage)
    where
        OrdersReader: Handler<ToReaderMessage>,
        ToReaderMessage: Message + Send + 'static,
        ToReaderMessage::Result: Send,
    {
        if let Err(e) = self.reader_addr.try_send(msg) {
            error!("[READER] Error sending message to coffee maker, stopping...");
            System::current().stop();
        }
    }
}
impl Actor for CoffeeMaker {
    type Context = Context<Self>;
}

impl Handler<ErrorOpeningFile> for CoffeeMaker {
    type Result = ();

    fn handle(&mut self, _msg: ErrorOpeningFile, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[COFFEE MAKER]");
    }
}

impl Handler<OpenedFile> for CoffeeMaker {
    type Result = ();

    fn handle(&mut self, _msg: OpenedFile, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[COFFEE MAKER] Received message to start reading orders");
        self.reader_addr.try_send(ReadAnOrder);
    }
}

impl Handler<FinishedFile> for CoffeeMaker {
    type Result = ();

    fn handle(&mut self, _msg: FinishedFile, ctx: &mut Context<Self>) -> Self::Result {
        debug!("[COFFEE MAKER] Received message to finish");
        ctx.stop();
        System::current().stop();
    }
}

impl Handler<ProcessOrder> for CoffeeMaker {
    type Result = ();

    fn handle(&mut self, msg: ProcessOrder, _ctx: &mut Context<Self>) -> Self::Result {
        let order = msg.0;
        debug!("Received order {:?}", order);
        match order.consumption_type {
            ConsumptionType::Cash => {
                let _success = self.order_randomizer.get_random_success();
                // TODO: consultar qué hacer si falla hacer el café con cash.
                let future = async move {
                    let result = self
                        .server_conn
                        .add_points(order.account_id, order.consumption)
                        .await;
                    self.handle_server_result(result);
                };
                Box::pin(future.into_actor(self).map(move |_result, _me, _ctx| {}))
            }
            ConsumptionType::Points => {
                let future = async move {
                    let result = self
                        .server_conn
                        .request_points(order.account_id, order.consumption)
                        .await;
                    match result {
                        Ok(()) => {
                            let success = self.order_randomizer.get_random_success();
                            if !success {
                                let result = self
                                    .server_conn
                                    .cancel_point_request(order.account_id)
                                    .await;
                                self.handle_server_result(result);
                                return;
                            }

                            let result = self
                                .server_conn
                                .take_points(order.account_id, order.consumption)
                                .await;
                            self.handle_server_result(result);
                        }
                        Err(ServerError::ConnectionLost) => {
                            error!("[CoffeeMaker] can't connect to server");
                            return;
                        }
                        Err(e) => {
                            error!("{:?}", e);
                            self.reader_addr.try_send(ReadAnOrder);
                            return;
                        }
                    }
                };
                Box::pin(future.into_actor(self).map(move |result, me, _ctx| {}))
            }
        }
    }
}

impl CoffeeMaker {
    fn handle_server_result(&mut self, result: Result<(), ServerError>) {
        match result {
            Err(e) => {
                error!("{:?}", e);
            }
            Ok(()) => {
                self.reader_addr.try_send(ReadAnOrder);
            }
        }
    }
}

fn set_logger_config() {
    if env::var("RUST_LOG").is_err() {
        if let Err(err) = simple_logger::init_with_level(log::Level::Debug) {
            println!("Error setting logger to default value. Error: {:?}", err);
        }
    } else if let Err(err) = simple_logger::init_with_env() {
        println!("Error setting logger: {:?}", err);
    }
}

pub fn main_coffee() {
    let system = System::new();
    set_logger_config();
    system.block_on(async {
        let reader = OrdersReader::new(String::from("tests/orders.csv"));
        let reader_addr = reader.start();
        let coffee_maker = CoffeeMaker {
            reader_addr: reader_addr.clone(),
            server_conn: Box::new(LocalServer {}),
            order_randomizer: Box::new(RealRandomizer::new(80)),
        };
        let coffee_maker_addr = coffee_maker.start();
        reader_addr.try_send(OpenFile(coffee_maker_addr));
    });

    system.run().unwrap();
}
