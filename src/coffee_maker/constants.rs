/// Indica el tiempo que tarda la cafetera en realizar un pedido en ms
pub const PROCESS_ORDER_TIME_IN_MS: u64 = 100;

/// Indica la probabilidad de exito de que la cafetera realice un pedido exitosamente. El valor debe estar en tre 0 y 100.
pub const SUCCESS_CHANCE: i32 = 80;

pub const DEFAULT_ORDERS_FILE: &str = "tests/orders.csv";
