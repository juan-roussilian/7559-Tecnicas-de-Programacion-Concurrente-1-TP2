use crate::coffee_maker::CoffeeMaker;
use crate::order::Order;
use actix::Addr;
use actix::Message;

#[derive(Message)]
#[rtype(result = "()")]
pub struct OpenFile(pub Addr<CoffeeMaker>);

#[derive(Message)]
#[rtype(result = "()")]
pub struct ReadAnOrder;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ProcessOrder(pub Order);

#[derive(Message)]
#[rtype(result = "()")]
pub struct OpenedFile;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ErrorOpeningFile;
