#[cfg(test)]
use mockall::automock;

use rand::Rng;

/// Interfaz del generador de chances de exito de un pedido
#[cfg_attr(test, automock)]
pub trait Randomizer {
    /// Retorna true o false de manera azarosa
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
    /// Retornara true o false dependiendo de un numero autogenerado al azar entre 0 y 100. Si este es
    /// es menor o igual que la el porcentaje de exito determinado al instanciar la clase
    fn get_random_success(&self) -> bool {
        let num = rand::thread_rng().gen_range(0, 100);
        num <= self.success_chance
    }
}
