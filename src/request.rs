use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use serde::{Serialize, Deserialize};
use serde_json::json;
use web_sys::{Request, RequestInit, RequestMode, Response, Headers};
use std::sync::OnceLock;

#[derive(Serialize, Deserialize)]
struct Shape {
    name: String,
    svg: String,
}

static SERVER_URL: OnceLock<String> = OnceLock::new();

#[wasm_bindgen]
pub fn set_server_url(url: String) {
    SERVER_URL.set(url).ok();
}

fn get_server_url() -> String {
    SERVER_URL.get().cloned().unwrap_or_else(|| "http://127.0.0.1:1073".to_string())
}

#[wasm_bindgen]
pub async fn save_svg(name: String, svg: String) -> Result<JsValue, JsValue> {
    let shape = Shape { name, svg };
    let body = serde_json::to_string(&shape).map_err(|e| JsValue::from_str(&e.to_string()))?;
    
    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.mode(RequestMode::Cors);
    opts.body(Some(&JsValue::from_str(&body)));
    
    let url = format!("{}/save_svg", get_server_url());
    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    
    request.headers().set("Content-Type", "application/json").map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    
    let window = web_sys::window().ok_or("No window object")?;
    let response = JsFuture::from(window.fetch_with_request(&request)).await?;
    let response: Response = response.dyn_into().map_err(|_| JsValue::from_str("Response conversion failed"))?;
    
    if response.ok() {
        Ok(JsValue::from_str("SVG saved successfully"))
    } else {
        Err(JsValue::from_str("Failed to save SVG"))
    }
}

#[wasm_bindgen]
pub async fn get_svgs() -> Result<JsValue, JsValue> {
    let url = format!("{}/get_svgs", get_server_url());
    let request = Request::new_with_str(&url)
        .map_err(|e| JsValue::from_str(&format!("{:?}", e)))?;
    
    let window = web_sys::window().ok_or("No window object")?;
    let response = JsFuture::from(window.fetch_with_request(&request)).await?;
    let response: Response = response.dyn_into().map_err(|_| JsValue::from_str("Response conversion failed"))?;
    
    let json = JsFuture::from(response.json().map_err(|e| JsValue::from_str(&format!("{:?}", e)))?).await?;
    Ok(json)
}