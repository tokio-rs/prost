#[derive(Debug, Message)]
pub struct WidgetFactory {
    #[proto(tag="1")]
    pub inner: Option<widget_factory::Inner>,
    #[proto(tag="2")]
    pub root: Option<super::super::Root>,
    #[proto(tag="3")]
    pub root_inner: Option<super::super::root::Inner>,
    #[proto(tag="4")]
    pub widget: Option<super::Widget>,
    #[proto(tag="5")]
    pub widget_inner: Option<super::widget::Inner>,
    #[proto(tag="6")]
    pub gizmo: Option<super::super::gizmo::Gizmo>,
    #[proto(tag="7")]
    pub gizmo_inner: Option<super::super::gizmo::gizmo::Inner>,
    #[proto(tag="8")]
    pub gizmo_factory: Option<super::super::gizmo::factory::GizmoFactory>,
    #[proto(tag="9")]
    pub gizmo_factory_inner: Option<super::super::gizmo::factory::gizmo_factory::Inner>,
}
pub mod widget_factory {
    #[derive(Debug, Message)]
    pub struct Inner {
    }
}
