use rand::Rng;

pub trait Randomizer {
    fn get_random_success(&self) -> bool;
}

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
        num >= self.success_chance
    }
}
