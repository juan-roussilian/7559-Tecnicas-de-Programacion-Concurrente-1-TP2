use crate::coffee_maker::CoffeeMaker;
use crate::order::Order;
use actix::Addr;
use actix::Message;

#[derive(Message)]
#[rtype(result = "()")]
pub struct OpenFile(pub Vec<Addr<CoffeeMaker>>);

#[derive(Message)]
#[rtype(result = "()")]
pub struct ReadAnOrder(pub usize);

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct ProcessOrder(pub Order);

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct OpenedFile;

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct ErrorOpeningFile;

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct FinishedFile;
