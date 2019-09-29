quick_error! {
    #[derive(Debug)]
    pub enum DStoreError {
        Io(err: ::std::io::Error) {
            description(err.description())
            display("[ERR]:IO error {}", err)
            cause(err)
            from()
        }
        // FIXME
        // None(err: ::std::option::NoneError) {
        //     from()
        // }
        Uuid(err: ::uuid::parser::ParseError) {
            description(err.description())
            display("[ERR] Uuid error {}", err)
            cause(err)
            from()
        }
         Serialize(err: super::json::SerializeError) {
            description(err.description())
            display("[ERR] Deserialize error: {}", err)
            cause(err)
            from()
        }
        Deserialize(err: super::json::DeserializeError) {
            from()
        }
        Poison {}
        NotFound {}
    }
}

impl<T> From<::std::sync::PoisonError<T>> for DStoreError {
    fn from(_: ::std::sync::PoisonError<T>) -> DStoreError {
        DStoreError::Poison
    }
}
