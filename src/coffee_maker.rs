use std::sync::Arc;

use actix::{
    Actor, ActorContext, ActorFutureExt, Addr, Context, Handler, Message, ResponseActFuture,
    WrapFuture,
};
use actix_rt::System;
use async_std::sync::Mutex;
use log::{debug, error};

use crate::errors::ServerError;
use crate::logger::set_logger_config;
use crate::messages::{
    ErrorOpeningFile, FinishedFile, OpenFile, OpenedFile, ProcessOrder, ReadAnOrder,
};
use crate::order::ConsumptionType;
use crate::orders_reader::OrdersReader;
use crate::randomizer::{Randomizer, RealRandomizer};
use crate::server::{LocalServer, Server};

pub struct CoffeeMaker {
    reader_addr: Addr<OrdersReader>,
    server_conn: Arc<Mutex<Box<dyn Server>>>,
    order_randomizer: Arc<Mutex<Box<dyn Randomizer>>>,
}

impl CoffeeMaker {
    fn new(
        reader_addr: Addr<OrdersReader>,
        _server_port: usize,
        order_randomizer: Box<dyn Randomizer>,
    ) -> CoffeeMaker {
        CoffeeMaker {
            reader_addr,
            server_conn: Arc::new(Mutex::new(Box::new(LocalServer {}))),
            order_randomizer: Arc::new(Mutex::new(order_randomizer)),
        }
    }

    fn send_message<ToReaderMessage>(&self, msg: ToReaderMessage)
    where
        OrdersReader: Handler<ToReaderMessage>,
        ToReaderMessage: Message + Send + 'static,
        ToReaderMessage::Result: Send,
    {
        if self.reader_addr.try_send(msg).is_err() {
            error!("[COFFEE MAKER] Error sending message to reader, stopping...");
            System::current().stop();
        }
    }

    fn handle_server_result(&mut self, result: Result<(), ServerError>, ctx: &mut Context<Self>) {
        match result {
            Err(ServerError::ConnectionLost) => {
                error!("[CoffeeMaker] can't connect to server,, stopping...");
                self.stop_system(ctx);
            }
            Err(e) => {
                error!("{:?}", e);
                self.send_message(ReadAnOrder);
            }
            Ok(()) => {
                self.send_message(ReadAnOrder);
            }
        }
    }

    fn stop_system(&mut self, ctx: &mut Context<Self>) {
        ctx.stop();
        System::current().stop();
    }
}

impl Actor for CoffeeMaker {
    type Context = Context<Self>;
}

impl Handler<ErrorOpeningFile> for CoffeeMaker {
    type Result = ();

    fn handle(&mut self, _msg: ErrorOpeningFile, ctx: &mut Context<Self>) -> Self::Result {
        debug!("[COFFEE MAKER] Received message of error opening file");
        self.stop_system(ctx)
    }
}

impl Handler<OpenedFile> for CoffeeMaker {
    type Result = ();

    fn handle(&mut self, _msg: OpenedFile, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[COFFEE MAKER] Received message to start reading orders");
        self.send_message(ReadAnOrder);
    }
}

impl Handler<FinishedFile> for CoffeeMaker {
    type Result = ();

    fn handle(&mut self, _msg: FinishedFile, ctx: &mut Context<Self>) -> Self::Result {
        debug!("[COFFEE MAKER] Received message to finish");
        self.stop_system(ctx)
    }
}

impl Handler<ProcessOrder> for CoffeeMaker {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: ProcessOrder, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Received order {:?}", msg.0);
        let order = msg.0;
        let randomizer = self.order_randomizer.clone();
        let server: Arc<Mutex<Box<dyn Server>>> = self.server_conn.clone();
        match order.consumption_type {
            ConsumptionType::Cash => {
                // TODO: consultar qué hacer si falla hacer el café con cash.
                let future = async move {
                    let _success = randomizer.lock().await.get_random_success();
                    let server_conn = server.lock().await;
                    server_conn
                        .add_points(order.account_id, order.consumption)
                        .await
                };
                Box::pin(future.into_actor(self).map(|result, me, ctx| {
                    me.handle_server_result(result, ctx);
                }))
            }
            ConsumptionType::Points => {
                let future = async move {
                    let result = server
                        .lock()
                        .await
                        .request_points(order.account_id, order.consumption)
                        .await;
                    if let Ok(()) = result {
                        let success = randomizer.lock().await.get_random_success();
                        if !success {
                            return server
                                .lock()
                                .await
                                .cancel_point_request(order.account_id)
                                .await;
                        }
                        return server
                            .lock()
                            .await
                            .take_points(order.account_id, order.consumption)
                            .await;
                    }
                    result
                };
                Box::pin(future.into_actor(self).map(|result, me, ctx| {
                    me.handle_server_result(result, ctx);
                }))
            }
        }
    }
}

pub fn main_coffee() {
    let system = System::new();
    set_logger_config();
    system.block_on(async {
        let reader = OrdersReader::new(String::from("tests/orders.csv"));
        let reader_addr = reader.start();
        let coffee_maker =
            CoffeeMaker::new(reader_addr.clone(), 8080, Box::new(RealRandomizer::new(80)));
        let coffee_maker_addr = coffee_maker.start();
        if reader_addr.try_send(OpenFile(coffee_maker_addr)).is_err() {
            error!("[COFFEE MAKER] Unable to send OpenFile message to file reader");
            System::current().stop();
        }
    });

    system.run().unwrap();
}
