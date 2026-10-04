use js_sys::Array;
use wasm_bindgen::JsValue;
use web_sys::DomStringList;

pub fn dom_string_list_to_vec(list: &DomStringList) -> Vec<String> {
    let mut vec = vec![];

    for i in 0..list.length() {
        if let Some(s) = list.get(i) {
            vec.push(s);
        }
    }

    vec.shrink_to_fit();
    vec
}

pub fn array_to_vec(array: Array) -> Vec<JsValue> {
    let mut vec = Vec::new();
    for i in 0..array.length() {
        vec.push(array.get(i));
    }
    vec
}
