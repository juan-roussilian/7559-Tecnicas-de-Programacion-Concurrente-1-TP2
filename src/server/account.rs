use chrono::{DateTime, Local};

pub struct Account{
    pub id: usize,
    amount: usize,
    last_updated_on: DateTime<Local>,
    is_reserved: bool
}

impl Account{
    pub fn new(id:usize, amount:usize)->Self{
        Account{id,amount, last_updated_on: Local::now(), is_reserved:false}
    }
    pub fn points(){

    }
    pub fn add_points(&mut self){
        return
    }
    pub fn update_points(&mut self){

    }
    pub fn substract_points(&mut self){

    }
    pub fn reserve(&mut self){

    }

}