use std::sync::Arc;

use std::sync::{Condvar, Mutex};

use crate::errors::ServerError;
use crate::orders_queue::OrdersQueue;

pub struct OrdersManager {
    orders: Arc<Mutex<OrdersQueue>>,
    orders_cond: Arc<Condvar>,
    // channel para recibir las respuestas y responder al request points
}

impl OrdersManager {
    pub fn new(orders: Arc<Mutex<OrdersQueue>>, orders_cond: Arc<Condvar>) -> OrdersManager {
        OrdersManager {
            orders,
            orders_cond,
        }
    }

    pub fn handle_orders(&mut self) -> Result<(), ServerError> {
        loop {
            let mut orders = self
                .orders_cond
                .wait_while(self.orders.lock()?, |orders| orders.is_empty())?;
            let adding_orders = orders.get_and_clear_adding_orders();
            for order in adding_orders {
                // TODO agregar puntos a la db local
                // TODO hacer broadcast de los cambios a los demas servidores
            }

            let request_points_orders = orders.get_and_clear_request_points_orders();
            let total_request_orders = request_points_orders.len();
            for order in request_points_orders {
                // TODO ver si alcanzan los puntos (si hay 2 o mas sobre la misma cuenta ir acumulando en el gestor de puntos?)
                // TODO responder si alcanza o no
            }

            for _ in 0..total_request_orders {
                // TODO recibir el resultado de la orden de resta
                // TODO restar los puntos locales si corresponde
                // TODO broadcast de la resta si corresponde
            }
            self.orders_cond.notify_all();
        }
    }
}
