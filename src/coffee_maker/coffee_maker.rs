use std::sync::Arc;

use actix::{
    Actor, ActorContext, ActorFutureExt, Addr, Context, Handler, Message, ResponseActFuture,
    WrapFuture,
};
use actix_rt::System;
use async_std::sync::Mutex;
use log::{debug, error};

use crate::actor_messages::{
    ErrorOpeningFile, FinishedFile, OpenedFile, ProcessOrder, ReadAnOrder,
};
use crate::local_server_client::{LocalServer, LocalServerClient};
use crate::order::{ConsumptionType, Order};
use crate::orders_reader::OrdersReader;
use crate::randomizer::Randomizer;
use lib::common_errors::ServerError;

pub struct CoffeeMaker {
    reader_addr: Addr<OrdersReader>,
    server_conn: Arc<Mutex<Box<dyn LocalServerClient>>>,
    order_randomizer: Arc<Mutex<Box<dyn Randomizer>>>,
}

impl CoffeeMaker {
    pub fn new(
        reader_addr: Addr<OrdersReader>,
        _server_port: usize,
        order_randomizer: Box<dyn Randomizer>,
    ) -> Result<CoffeeMaker, ServerError> {
        let connection = LocalServer::new()?;
        Ok(CoffeeMaker {
            reader_addr,
            server_conn: Arc::new(Mutex::new(Box::new(connection))),
            order_randomizer: Arc::new(Mutex::new(order_randomizer)),
        })
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
        let server = self.server_conn.clone();
        match order.consumption_type {
            ConsumptionType::Cash => {
                Box::pin(add_points(order, server, randomizer).into_actor(self).map(
                    |result, me, ctx| {
                        me.handle_server_result(result, ctx);
                    },
                ))
            }
            ConsumptionType::Points => Box::pin(
                consume_points(order, server, randomizer)
                    .into_actor(self)
                    .map(|result, me, ctx| {
                        me.handle_server_result(result, ctx);
                    }),
            ),
        }
    }
}

async fn add_points(
    order: Order,
    server: Arc<Mutex<Box<dyn LocalServerClient>>>,
    randomizer: Arc<Mutex<Box<dyn Randomizer>>>,
) -> Result<(), ServerError> {
    // TODO: consultar qué hacer si falla hacer el café con cash.
    let _success = randomizer.lock().await.get_random_success();
    let server_conn = server.lock().await;
    server_conn
        .add_points(order.account_id, order.consumption)
        .await
}

async fn consume_points(
    order: Order,
    server: Arc<Mutex<Box<dyn LocalServerClient>>>,
    randomizer: Arc<Mutex<Box<dyn Randomizer>>>,
) -> Result<(), ServerError> {
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
}

#[cfg(test)]
mod tests {
    use crate::{
        local_server_client::MockLocalServerClient, order::ConsumptionType,
        randomizer::MockRandomizer,
    };

    use super::*;

    #[actix_rt::test]
    async fn should_add_points_to_account() {
        let order = Order {
            account_id: 100,
            consumption: 1000,
            consumption_type: ConsumptionType::Cash,
        };

        let mut rand_mock = MockRandomizer::new();
        rand_mock.expect_get_random_success().returning(|| true);
        let rand_mock: Arc<Mutex<Box<dyn Randomizer>>> = Arc::new(Mutex::new(Box::new(rand_mock)));

        let mut connection_mock = MockLocalServerClient::new();
        connection_mock.expect_add_points().returning(|_, _| Ok(()));
        let connection_mock: Arc<Mutex<Box<dyn LocalServerClient>>> =
            Arc::new(Mutex::new(Box::new(connection_mock)));
        let result = add_points(order, connection_mock.clone(), rand_mock.clone()).await;
        assert!(result.is_ok());
    }

    #[actix_rt::test]
    async fn should_return_server_error_when_adding_points_if_the_connection_is_lost() {
        let order = Order {
            account_id: 100,
            consumption: 1000,
            consumption_type: ConsumptionType::Cash,
        };

        let mut rand_mock = MockRandomizer::new();
        rand_mock.expect_get_random_success().returning(|| true);
        let rand_mock: Arc<Mutex<Box<dyn Randomizer>>> = Arc::new(Mutex::new(Box::new(rand_mock)));

        let mut connection_mock = MockLocalServerClient::new();
        connection_mock
            .expect_add_points()
            .returning(|_, _| Err(ServerError::ConnectionLost));
        let connection_mock: Arc<Mutex<Box<dyn LocalServerClient>>> =
            Arc::new(Mutex::new(Box::new(connection_mock)));
        let result = add_points(order, connection_mock.clone(), rand_mock.clone()).await;
        assert!(result.is_err());
        assert_eq!(ServerError::ConnectionLost, result.unwrap_err());
    }

    #[actix_rt::test]
    async fn should_return_ok_if_the_order_fails_to_be_processed_when_using_points() {
        let order = Order {
            account_id: 100,
            consumption: 1000,
            consumption_type: ConsumptionType::Points,
        };

        let mut rand_mock = MockRandomizer::new();
        rand_mock.expect_get_random_success().returning(|| false);
        let rand_mock: Arc<Mutex<Box<dyn Randomizer>>> = Arc::new(Mutex::new(Box::new(rand_mock)));

        let mut connection_mock = MockLocalServerClient::new();
        connection_mock
            .expect_request_points()
            .returning(|_, _| Ok(()));
        connection_mock
            .expect_cancel_point_request()
            .returning(|_| Ok(()));
        let connection_mock: Arc<Mutex<Box<dyn LocalServerClient>>> =
            Arc::new(Mutex::new(Box::new(connection_mock)));
        let result = consume_points(order, connection_mock.clone(), rand_mock.clone()).await;
        assert!(result.is_ok());
    }

    #[actix_rt::test]
    async fn should_return_server_error_if_the_order_is_successful_but_lost_connection_when_using_points(
    ) {
        let order = Order {
            account_id: 100,
            consumption: 1000,
            consumption_type: ConsumptionType::Points,
        };

        let mut rand_mock = MockRandomizer::new();
        rand_mock.expect_get_random_success().returning(|| true);
        let rand_mock: Arc<Mutex<Box<dyn Randomizer>>> = Arc::new(Mutex::new(Box::new(rand_mock)));

        let mut connection_mock = MockLocalServerClient::new();
        connection_mock
            .expect_request_points()
            .returning(|_, _| Ok(()));
        connection_mock
            .expect_take_points()
            .returning(|_, _| Err(ServerError::ConnectionLost));
        let connection_mock: Arc<Mutex<Box<dyn LocalServerClient>>> =
            Arc::new(Mutex::new(Box::new(connection_mock)));
        let result = consume_points(order, connection_mock.clone(), rand_mock.clone()).await;
        assert!(result.is_err());
        assert_eq!(ServerError::ConnectionLost, result.unwrap_err());
    }

    #[actix_rt::test]
    async fn should_return_not_enough_points_when_using_points() {
        let order = Order {
            account_id: 100,
            consumption: 1000,
            consumption_type: ConsumptionType::Points,
        };

        let rand_mock = MockRandomizer::new();
        let rand_mock: Arc<Mutex<Box<dyn Randomizer>>> = Arc::new(Mutex::new(Box::new(rand_mock)));

        let mut connection_mock = MockLocalServerClient::new();
        connection_mock
            .expect_request_points()
            .returning(|_, _| Err(ServerError::NotEnoughPoints));
        let connection_mock: Arc<Mutex<Box<dyn LocalServerClient>>> =
            Arc::new(Mutex::new(Box::new(connection_mock)));
        let result = consume_points(order, connection_mock.clone(), rand_mock.clone()).await;
        assert!(result.is_err());
        assert_eq!(ServerError::NotEnoughPoints, result.unwrap_err());
    }
}
