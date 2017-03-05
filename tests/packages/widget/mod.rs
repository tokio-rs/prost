pub mod factory;
#[derive(Debug, Message)]
pub struct Widget {
}
pub mod widget {
    #[derive(Debug, Message)]
    pub struct Inner {
    }
}
