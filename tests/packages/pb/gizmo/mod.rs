pub mod factory;
#[derive(Debug, Message)]
pub struct Gizmo {
}
pub mod gizmo {
    #[derive(Debug, Message)]
    pub struct Inner {
    }
}
