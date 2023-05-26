use crate::coffee_maker::CoffeeMaker;
use crate::messages::{ErrorOpeningFile, OpenFile, OpenedFile, ReadAnOrder};
use actix::{Actor, Addr, Context, Handler, ResponseActFuture, WrapFuture};
use async_std::fs::File;
use async_std::io::prelude::BufReadExt;
use async_std::io::BufReader;
use log::{debug, error, info};

pub struct OrdersReader {
    pub(crate) file_name: String,
    pub(crate) file: Option<BufReader<File>>,
    pub(crate) coffee_maker_addr: Option<Addr<CoffeeMaker>>,
    pub(crate) line: String,
}

impl Actor for OrdersReader {
    type Context = Context<Self>;
}

impl Handler<OpenFile> for OrdersReader {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: OpenFile, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[READER] Open file started");
        self.coffee_maker_addr = Some(msg.0);
        Box::pin(File::open(self.file_name.clone()).into_actor(self).map(
            move |result, me, _ctx| {
                if result.is_err() {
                    error!("[READER] Error opening file: {}", me.file_name);
                    me.coffee_maker_addr.try_send(ErrorOpeningFile);
                    return;
                }
                info!("[READER] Opened file: {}", me.file_name);
                me.file = Some(BufReader::new(result.unwrap()));
                me.coffee_maker_addr.try_send(OpenedFile);
            },
        ))
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
                }),
        )
    }
}
