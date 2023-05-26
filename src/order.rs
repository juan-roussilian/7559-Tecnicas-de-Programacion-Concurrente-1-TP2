pub struct Order {
    pub consumption_type: ConsumptionType,
    pub account_id: usize,
    pub consumption: usize,
}

pub enum ConsumptionType {
    Points,
    Cash,
}
