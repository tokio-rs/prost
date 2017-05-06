pub mod factory;
#[derive(Debug, Message, PartialEq)]
pub struct Gizmo {
}
pub mod gizmo {
    #[derive(Debug, Message, PartialEq)]
    pub struct Inner {
    }
}
