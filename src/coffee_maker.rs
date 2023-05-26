use actix::dev::SendError;
use actix::{
    Actor, ActorFutureExt, Addr, Context, Handler, ResponseActFuture, WrapFuture,
};
use actix_rt::System;
use log::{debug, error};
use std::env;

use crate::errors::{ServerError};
use crate::messages::{ErrorOpeningFile, OpenFile, OpenedFile, ProcessOrder, ReadAnOrder};
use crate::order::{ConsumptionType};
use crate::orders_reader::OrdersReader;
use crate::randomizer::{Randomizer, RealRandomizer};
use crate::server::Server;

pub struct CoffeeMaker {
    reader_addr: Addr<OrdersReader>,
    server_conn: Box<dyn Server>,
    order_randomizer: Box<dyn Randomizer>,
}

impl Actor for CoffeeMaker {
    type Context = Context<Self>;
}

impl Handler<ErrorOpeningFile> for CoffeeMaker {
    type Result = ();

    fn handle(&mut self, msg: ErrorOpeningFile, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[COFFEE MAKER]");
    }
}

impl Handler<OpenedFile> for CoffeeMaker {
    type Result = ();

    fn handle(&mut self, msg: OpenedFile, _ctx: &mut Context<Self>) -> Self::Result {
        self.reader_addr.try_send(ReadAnOrder);
    }
}

impl Handler<ProcessOrder> for CoffeeMaker {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: ProcessOrder, _ctx: &mut Context<Self>) -> Self::Result {
        match msg.0.consumption_type {
            ConsumptionType::Cash => {
                let _success = self.order_randomizer.get_random_success();
                // TODO: consultar qué hacer si falla hacer el café con cash.
                Box::pin(
                    self.server_conn
                        .add_points(msg.0.account_id, msg.0.consumption)
                        .into_actor(self)
                        .map(handle_server_result),
                )
            }
            ConsumptionType::Points => Box::pin(
                self.server_conn
                    .request_points(msg.0.account_id, msg.0.consumption)
                    .into_actor(self)
                    .map(move |result, me, _ctx| match result {
                        Ok(()) => {
                            let success = self.order_randomizer.get_random_success();
                            if !success {
                                Box::pin(
                                    self.server_conn
                                        .cancel_point_request(msg.0.account_id)
                                        .into_actor()
                                        .map(handle_server_result),
                                )
                            }

                            Box::pin(
                                self.server_conn
                                    .take_points(msg.0.account_id, msg.0.consumption)
                                    .into_actor()
                                    .map(handle_server_result),
                            )
                        }
                        Err(ServerError::ConnectionLost) => {
                            error!("[CoffeeMaker] can't connect to server");
                        }
                        Err(e) => {
                            error!("{:?}", e);
                            self.reader_addr.try_send(ReadAnOrder)
                        }
                    }),
            ),
        }
    }
}

fn handle_server_result(
    result: Result<(), ServerError>,
    coffee_maker: &mut CoffeeMaker,
    _ctx: &mut Context<CoffeeMaker>,
) -> Result<(), SendError<ReadAnOrder>> {
    match result {
        Err(e) => {
            error!("{:?}", e);
            Ok(())
        }
        Ok(()) => coffee_maker.reader_addr.try_send(ReadAnOrder),
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
        let reader = OrdersReader {
            file_name: String::from("tests/orders.csv"),
            file: None,
            line: String::new(),
            coffee_maker_addr: None,
        };
        let reader_addr = reader.start();
        let coffee_maker = CoffeeMaker {
            reader_addr,
            server_conn: (),
            order_randomizer: RealRandomizer::new(80),
        };
        let coffee_maker_addr = coffee_maker.start();
        reader_addr.try_send(OpenFile(coffee_maker_addr));
    });

    system.run().unwrap();
}
