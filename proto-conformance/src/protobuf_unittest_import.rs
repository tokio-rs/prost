#[derive(Clone, Debug, PartialEq, Message)]
pub struct PublicImportMessage {
    #[proto(tag="1")]
    pub e: i32,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct ImportMessage {
    #[proto(tag="1")]
    pub d: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
pub enum ImportEnum {
    ImportEnumUnspecified = 0,
    ImportFoo = 7,
    ImportBar = 8,
    ImportBaz = 9,
}
