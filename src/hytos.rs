use std::{mem::offset_of, str};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{window, DragEvent, File, FileReader};
use js_sys::{Uint8Array, ArrayBuffer};
use diesel_wasm_sqlite::{Connection, QueryResult};
use diesel::prelude::*;
use serde::Serialize;
use log::info;

mod schema {
    table! {
        users (id) {
            id -> Integer,
            name -> Text,
        }
    }
}

#[derive(Queryable, Serialize)]
struct User {
    id: i32,
    name: String,
}

pub async fn read_sqlite_file(file: File) {
    let reader = FileReader::new().unwrap();
    let reader_clone = reader.clone();
    let file_clone = file.clone();

    let promise = JsFuture::new(&reader_clone);
    reader.read_as_array_buffer(&file_clone).unwrap();

    let result = promise.await.unwrap();
    let array_buffer: ArrayBuffer = result.unchecked_into();
    let uint8_array = Uint8Array::new(&array_buffer);
    let bytes = uint8_array.to_vec();

    match Connection::open_in_memory() {
        Ok(conn) => {
            conn.load_from_bytes(&bytes).unwrap();

            let users: QueryResult<Vec<User>> = conn
                .execute(|conn| schema::users::table.load::<User>(conn));

            match users {
                Ok(users) => {
                    let users_json = serde_wasm_bindgen::to_value(&users).unwrap();
                    web_sys::console::log_1(&users_json);
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Query Error: {:?}", e).into());
                }
            }
        }
        Err(e) => {
            web_sys::console::error_1(&format!("SQLite Error: {:?}", e).into());
        }
    }
}