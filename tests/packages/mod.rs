pub mod gizmo;
pub mod widget;
#[derive(Debug, Message)]
pub struct Root {
}
pub mod root {
    #[derive(Debug, Message)]
    pub struct Inner {
    }
}
