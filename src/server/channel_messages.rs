pub struct PointsRequest {
    pub coffee_maker_id: usize,
    pub account_id: usize,
}

pub struct TakePoints {
    pub points: usize,
    pub account_id: usize,
}
