use plist::plist;

fn dict() {
    plist!({
        "a" : true,
        "b" : "astring",
        "c" : 1,
        "d" : 1.0,
        "e" : [1, 2, 3],
        "f" : { "a" : 1, "b" : 2 },
    });
}
