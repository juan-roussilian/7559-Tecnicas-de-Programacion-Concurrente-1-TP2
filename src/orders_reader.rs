use std::rc::Rc;

use crate::coffee_maker::CoffeeMaker;
use crate::messages::{ ErrorOpeningFile, OpenFile, OpenedFile, ReadAnOrder };
use actix::{
    Actor,
    Addr,
    Context,
    Handler,
    ResponseActFuture,
    WrapFuture,
    ActorFutureExt,
    ResponseFuture,
};
use async_std::fs::File;
use async_std::io::prelude::BufReadExt;
use async_std::io::BufReader;
use async_std::sync::Mutex;
use log::{ debug, error, info };

pub struct OrdersReader {
    file_name: String,
    file: Option<Rc<Mutex<BufReader<File>>>>,
    coffee_maker_addr: Option<Addr<CoffeeMaker>>,
}

impl OrdersReader {
    pub fn new(file_name: String) -> OrdersReader {
        OrdersReader {
            file: None,
            file_name,
            coffee_maker_addr: None,
        }
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
                .map(move |result, me, _ctx| {
                    if result.is_err() {
                        error!("[READER] Error opening file: {}", me.file_name);
                        me.coffee_maker_addr
                            .as_ref()
                            .expect("Should not happen")
                            .try_send(ErrorOpeningFile);
                        return;
                    }
                    info!("[READER] Opened file: {}", me.file_name);
                    me.file = Some(Rc::new(Mutex::new(BufReader::new(result.unwrap()))));
                    me.coffee_maker_addr.as_ref().expect("Should not happen").try_send(OpenedFile);
                })
        )
    }
}

impl Handler<ReadAnOrder> for OrdersReader {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: ReadAnOrder, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[READER] Received message to read an order from the file");
        let file = self.file.as_mut().expect("Should be open already").clone();
        let future = async move {
            let mut file = file.lock().await;
            let mut line = String::new();
            let result = file.read_line(&mut line).await;
            if result.is_err() {
                error!("[READER] Error reading file");
                // handle error
                return;
            }
            info!("[READER] Line read from file: {}", line);
        };

        Box::pin(future)
    }
}
