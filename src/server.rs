use async_trait::async_trait;

use crate::errors::ServerError;

#[async_trait]
pub trait Server {
    async fn add_points(&self, account_id: usize, points: usize) -> Result<(), ServerError>;
    async fn request_points(&self, account_id: usize, points: usize) -> Result<(), ServerError>;
    async fn take_points(&self, account_id: usize, points: usize) -> Result<(), ServerError>;
    async fn cancel_point_request(&self, account_id: usize) -> Result<(), ServerError>;
}

pub struct LocalServer {}

impl Server for LocalServer {
    fn add_points<'life0, 'async_trait>(
        &'life0 self,
        account_id: usize,
        points: usize
    )
        -> core::pin::Pin<
            Box<
                dyn core::future::Future<Output = Result<(), ServerError>> +
                    core::marker::Send +
                    'async_trait
            >
        >
        where 'life0: 'async_trait, Self: 'async_trait
    {
        todo!()
    }

    fn request_points<'life0, 'async_trait>(
        &'life0 self,
        account_id: usize,
        points: usize
    )
        -> core::pin::Pin<
            Box<
                dyn core::future::Future<Output = Result<(), ServerError>> +
                    core::marker::Send +
                    'async_trait
            >
        >
        where 'life0: 'async_trait, Self: 'async_trait
    {
        todo!()
    }

    fn take_points<'life0, 'async_trait>(
        &'life0 self,
        account_id: usize,
        points: usize
    )
        -> core::pin::Pin<
            Box<
                dyn core::future::Future<Output = Result<(), ServerError>> +
                    core::marker::Send +
                    'async_trait
            >
        >
        where 'life0: 'async_trait, Self: 'async_trait
    {
        todo!()
    }

    fn cancel_point_request<'life0, 'async_trait>(
        &'life0 self,
        account_id: usize
    )
        -> core::pin::Pin<
            Box<
                dyn core::future::Future<Output = Result<(), ServerError>> +
                    core::marker::Send +
                    'async_trait
            >
        >
        where 'life0: 'async_trait, Self: 'async_trait
    {
        todo!()
    }
}
