pub trait MessageNamed {
    fn fqname() -> &'static str;
}

impl<M> MessageNamed for Box<M>
where
    M: MessageNamed,
{
    fn fqname() -> &'static str {
        M::fqname()
    }
}
