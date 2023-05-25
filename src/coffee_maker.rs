use actix_rt::System;
use async_std::{ fs::File, io::{ BufReader, prelude::BufReadExt } };
use std::{ env };
use actix::{
    Actor,
    Context,
    Handler,
    ResponseActFuture,
    Message,
    Addr,
    WrapFuture,
    ActorFutureExt,
};
use log::{ debug, error, info };

use crate::errors::CoffeeMakerError;

pub struct OrdersReader {
    file_name: String,
    file: Option<BufReader<File>>,
    coffee_maker_addr: Addr<CoffeeMaker>,
    line: String,
}

pub struct CoffeeMaker {
    reader_addr: Option<Addr<OrdersReader>>,
}

impl Actor for OrdersReader {
    type Context = Context<Self>;
}

impl Actor for CoffeeMaker {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OpenFile;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ReadAnOrder;

#[derive(Message)]
#[rtype(result = "()")]
pub struct OpenedFile;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ErrorOpeningFile;

impl Handler<OpenFile> for OrdersReader {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: OpenFile, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[READER] Open file started");

        Box::pin(
            File::open(self.file_name.clone())
                .into_actor(self)
                .map(move |result, me, _ctx| {
                    if result.is_err() {
                        error!("[READER] Error opening file: {}", me.file_name);
                        me.coffee_maker_addr.try_send(ErrorOpeningFile);
                        return;
                    }
                    info!("[READER] Opened file: {}", me.file_name);
                    me.file = Some(BufReader::new(result.unwrap()));
                    me.coffee_maker_addr.try_send(OpenedFile);
                })
        )
    }
}

impl Handler<ReadAnOrder> for OrdersReader {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: ReadAnOrder, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[READER] Received message to read an order from the file");
        //let file = &mut self.file.as_ref().expect("Should be open already");
        Box::pin(
            self.file
                .as_mut()
                .expect("Should be open already")
                .read_line(&mut self.line)
                .into_actor(self)
                .map(move |result, me, _ctx| {
                    if result.is_err() {
                        error!("[READER] Error reading file");
                        // handle error
                        return;
                    }
                    info!("[READER] Line read from file: {}", 2);

                    // parse line and send response
                })
        )
    }
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
        debug!("[COFFEE MAKER]");
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
        let coffee_maker = CoffeeMaker { reader_addr: None };
        let coffee_maker_addr = coffee_maker.start();
        let reader = OrdersReader {
            file_name: String::from("tests/orders.csv"),
            file: None,
            line: String::new(),
            coffee_maker_addr,
        };
        let reader_addr = reader.start();
        //coffee_maker.reader_addr = Some(reader_addr.clone()); enviar addr
        reader_addr.try_send(OpenFile)
    });

    system.run().unwrap();
}
