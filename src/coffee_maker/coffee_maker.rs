use std::sync::Arc;
use std::time::Duration;

use actix::{
    Actor, ActorContext, ActorFutureExt, Addr, Context, Handler, Message, ResponseActFuture,
    WrapFuture,
};
use actix_rt::System;
use async_std::sync::Mutex;
use log::{debug, error};

use crate::actor_messages::{OpenedFile, ProcessOrder, ReadAnOrder};
use crate::constants::PROCESS_ORDER_TIME_IN_MS;
use crate::local_server_client::{LocalServer, LocalServerClient};
use crate::order::{ConsumptionType, Order};
use crate::orders_reader::OrdersReader;
use crate::randomizer::Randomizer;
use lib::common_errors::CoffeeSystemError;

use self::sync::sleep;

mod sync {
    use std::time::Duration;

    #[cfg(not(test))]
    pub(crate) async fn sleep(d: Duration) {
        use async_std::task;
        task::sleep(d).await;
    }

    #[cfg(test)]
    pub(crate) async fn sleep(_: Duration) {
        use async_std::future::ready;
        ready(0).await;
    }
}

/// Representa a una cafetera que procesa los pedidos. Tiene la direccion del lector,
/// la conexion con el servidor, un generador de exitos de pedidos y su id
pub struct CoffeeMaker {
    reader_addr: Addr<OrdersReader>,
    server_conn: Arc<Mutex<Box<dyn LocalServerClient>>>,
    order_randomizer: Arc<Mutex<Box<dyn Randomizer>>>,
    id: usize,
}

impl CoffeeMaker {
    pub fn new(
        reader_addr: Addr<OrdersReader>,
        server_addr: &String,
        order_randomizer: Box<dyn Randomizer>,
        id: usize,
    ) -> Result<CoffeeMaker, CoffeeSystemError> {
        let connection = LocalServer::new(server_addr)?;
        Ok(CoffeeMaker {
            reader_addr,
            server_conn: Arc::new(Mutex::new(Box::new(connection))),
            order_randomizer: Arc::new(Mutex::new(order_randomizer)),
            id,
        })
    }
    
    fn send_message<ToReaderMessage>(&self, msg: ToReaderMessage)
    where
        OrdersReader: Handler<ToReaderMessage>,
        ToReaderMessage: Message + Send + 'static,
        ToReaderMessage::Result: Send,
    {
        if self.reader_addr.try_send(msg).is_err() {
            error!(
                "[COFFEE MAKER {}] Error sending message to reader, stopping...",
                self.id
            );
            System::current().stop();
        }
    }
    /// Handler de resultados que retorna el servidor a traves de su cliente y detiene la cafetera
    /// en caso de no poder conectarse a este
    fn handle_server_result(
        &mut self,
        result: Result<(), CoffeeSystemError>,
        ctx: &mut Context<Self>,
    ) {
        match result {
            Err(CoffeeSystemError::ConnectionLost) => {
                error!(
                    "[COFFEE MAKER {}] can't connect to server, stopping...",
                    self.id
                );
                self.stop_system(ctx);
            }
            Err(e) => {
                error!("{:?}", e);
                self.send_message(ReadAnOrder(self.id));
            }
            Ok(()) => {
                self.send_message(ReadAnOrder(self.id));
            }
        }
    }
    /// Detiene el sistema de la cafetera
    fn stop_system(&mut self, ctx: &mut Context<Self>) {
        ctx.stop();
        System::current().stop();
    }
}

impl Actor for CoffeeMaker {
    type Context = Context<Self>;
}

impl Handler<OpenedFile> for CoffeeMaker {
    type Result = ();

    fn handle(&mut self, _msg: OpenedFile, _ctx: &mut Context<Self>) -> Self::Result {
        debug!(
            "[COFFEE MAKER {}] Received message to start reading orders",
            self.id
        );
        self.send_message(ReadAnOrder(self.id));
    }
}

impl Handler<ProcessOrder> for CoffeeMaker {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: ProcessOrder, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("[COFFEE MAKER {}] Processing order: {:?}", self.id, msg.0);
        let order = msg.0;
        let randomizer = self.order_randomizer.clone();
        let server = self.server_conn.clone();
        match order.consumption_type {
            ConsumptionType::Cash => Box::pin(
                add_points(order, server, randomizer, self.id)
                    .into_actor(self)
                    .map(|result, me, ctx| {
                        me.handle_server_result(result, ctx);
                    }),
            ),
            ConsumptionType::Points => Box::pin(
                consume_points(order, server, randomizer, self.id)
                    .into_actor(self)
                    .map(|result, me, ctx| {
                        me.handle_server_result(result, ctx);
                    }),
            ),
        }
    }
}
/// Metodo para comunicar al actor de cliente de servidor que debe sumar puntos a una cuenta del servidor
async fn add_points(
    order: Order,
    server: Arc<Mutex<Box<dyn LocalServerClient>>>,
    randomizer: Arc<Mutex<Box<dyn Randomizer>>>,
    id: usize,
) -> Result<(), CoffeeSystemError> {
    sleep(Duration::from_millis(PROCESS_ORDER_TIME_IN_MS)).await;
    let success = randomizer.lock().await.get_random_success();
    if !success {
        debug!("[COFFEE MAKER {}] Failed to process order of cash", id);
        return Ok(());
    }
    let server_conn = server.lock().await;
    server_conn
        .add_points(order.account_id, order.consumption)
        .await
}
/// Metodo para comunicar al actor de cliente de servidor que han sido consumidos puntos al servidor
/// en caso de que la orden sea producida correctamente
async fn consume_points(
    order: Order,
    server: Arc<Mutex<Box<dyn LocalServerClient>>>,
    randomizer: Arc<Mutex<Box<dyn Randomizer>>>,
    id: usize,
) -> Result<(), CoffeeSystemError> {
    let result = server
        .lock()
        .await
        .request_points(order.account_id, order.consumption)
        .await;
    if let Ok(()) = result {
        sleep(Duration::from_millis(PROCESS_ORDER_TIME_IN_MS)).await;
        let success = randomizer.lock().await.get_random_success();
        if !success {
            debug!("[COFFEE MAKER {}] Failed to process order of points", id);
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
        let result = add_points(order, connection_mock.clone(), rand_mock.clone(), 0).await;
        assert!(result.is_ok());
    }

    #[actix_rt::test]
    async fn should_return_ok_if_the_coffee_fails_when_adding_points() {
        let order = Order {
            account_id: 100,
            consumption: 1000,
            consumption_type: ConsumptionType::Cash,
        };

        let mut rand_mock = MockRandomizer::new();
        rand_mock.expect_get_random_success().returning(|| false);
        let rand_mock: Arc<Mutex<Box<dyn Randomizer>>> = Arc::new(Mutex::new(Box::new(rand_mock)));

        let connection_mock = MockLocalServerClient::new();
        let connection_mock: Arc<Mutex<Box<dyn LocalServerClient>>> =
            Arc::new(Mutex::new(Box::new(connection_mock)));
        let result = add_points(order, connection_mock.clone(), rand_mock.clone(), 0).await;
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
            .returning(|_, _| Err(CoffeeSystemError::ConnectionLost));
        let connection_mock: Arc<Mutex<Box<dyn LocalServerClient>>> =
            Arc::new(Mutex::new(Box::new(connection_mock)));
        let result = add_points(order, connection_mock.clone(), rand_mock.clone(), 0).await;
        assert!(result.is_err());
        assert_eq!(CoffeeSystemError::ConnectionLost, result.unwrap_err());
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
        let result = consume_points(order, connection_mock.clone(), rand_mock.clone(), 0).await;
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
            .returning(|_, _| Err(CoffeeSystemError::ConnectionLost));
        let connection_mock: Arc<Mutex<Box<dyn LocalServerClient>>> =
            Arc::new(Mutex::new(Box::new(connection_mock)));
        let result = consume_points(order, connection_mock.clone(), rand_mock.clone(), 0).await;
        assert!(result.is_err());
        assert_eq!(CoffeeSystemError::ConnectionLost, result.unwrap_err());
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
            .returning(|_, _| Err(CoffeeSystemError::NotEnoughPoints));
        let connection_mock: Arc<Mutex<Box<dyn LocalServerClient>>> =
            Arc::new(Mutex::new(Box::new(connection_mock)));
        let result = consume_points(order, connection_mock.clone(), rand_mock.clone(), 0).await;
        assert!(result.is_err());
        assert_eq!(CoffeeSystemError::NotEnoughPoints, result.unwrap_err());
    }
}
