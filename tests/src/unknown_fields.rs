#[derive(Clone, PartialEq, Message)]
pub struct AMessage {
    #[prost(string, tag="1")]
    pub name: String,
    #[prost(int32, tag="2")]
    pub id: i32,
    #[prost(string, tag="3")]
    pub email: String,
    #[prost(unknown_field_set)]
    pub unknown_fields: ::prost::UnknownFieldSet,
}
