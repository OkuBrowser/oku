use plist::plist;
fn dict() {
    ::plist::Value::Dictionary({
        let mut object = ::plist::Dictionary::new();
        let _ = object.insert(("a").into(), ::plist::Value::Boolean(true));
        let _ = object.insert(("b").into(), ::plist::Value::from("astring"));
        let _ = object.insert(("c").into(), ::plist::Value::from(1));
        let _ = object.insert(("d").into(), ::plist::Value::from(1.0));
        let _ = object
            .insert(
                ("e").into(),
                ::plist::Value::Array(
                    <[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            ::plist::Value::from(1),
                            ::plist::Value::from(2),
                            ::plist::Value::from(3),
                        ]),
                    ),
                ),
            );
        let _ = object
            .insert(
                ("f").into(),
                ::plist::Value::Dictionary({
                    let mut object = ::plist::Dictionary::new();
                    let _ = object.insert(("a").into(), ::plist::Value::from(1));
                    let _ = object.insert(("b").into(), ::plist::Value::from(2));
                    object
                }),
            );
        object
    });
}
