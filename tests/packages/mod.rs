pub mod gizmo;
pub mod widget;
#[derive(Debug, Message, PartialEq)]
pub struct Root {
}
pub mod root {
    #[derive(Debug, Message, PartialEq)]
    pub struct Inner {
    }
}
