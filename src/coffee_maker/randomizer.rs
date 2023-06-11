#[cfg(test)]
use mockall::automock;

use rand::Rng;

/// Interfaz del generador de chances de exito de un pedido
#[cfg_attr(test, automock)]
pub trait Randomizer {
    fn get_random_success(&self) -> bool;
}

/// Generador de chances de exito de un pedido real.
pub struct RealRandomizer {
    success_chance: i32,
}

impl RealRandomizer {
    pub fn new(success_chance: i32) -> Self {
        Self { success_chance }
    }
}

impl Randomizer for RealRandomizer {
    fn get_random_success(&self) -> bool {
        let num = rand::thread_rng().gen_range(0, 100);
        num <= self.success_chance
    }
}
