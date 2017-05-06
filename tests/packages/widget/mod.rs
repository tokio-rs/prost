pub mod factory;
#[derive(Debug, Message, PartialEq)]
pub struct Widget {
}
pub mod widget {
    #[derive(Debug, Message, PartialEq)]
    pub struct Inner {
    }
}
