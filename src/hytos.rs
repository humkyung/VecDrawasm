use std::{mem::offset_of, str};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{DragEvent, File, FileReader};
use js_sys::{Promise, Uint8Array, ArrayBuffer};
use serde::Serialize;
use log::info;

pub async fn read_sqlite_file(file: File) {
    let reader = FileReader::new().unwrap();

    let promise = JsFuture::from(Promise::new(&mut |resolve, _| {
        let onload_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            resolve.call0(&JsValue::null()).unwrap();
        }) as Box<dyn FnMut(_)>);

        reader.set_onload(Some(onload_closure.as_ref().unchecked_ref()));
        onload_closure.forget(); // Prevent Rust from dropping the closure
    }));

    reader.read_as_array_buffer(&file).unwrap();
    promise.await.unwrap(); // Wait for the file to finish loading

    let array_buffer: ArrayBuffer = reader.result().unwrap().unchecked_into();
    let uint8_array = Uint8Array::new(&array_buffer);
    let bytes = uint8_array.to_vec();

    web_sys::console::log_1(&format!("Loaded SQLite DB from file: {}", file.name()).into());
}