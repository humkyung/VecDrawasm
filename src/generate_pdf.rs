use wasm_bindgen::prelude::*;
use std::io::Cursor;

/// Piet 스타일의 캔버스를 SVG로 변환하는 함수 (샘플 SVG 포함)
#[wasm_bindgen]
pub fn generate_piet_svg() -> String {
    let svg = r#"
        <svg width="210mm" height="297mm" viewBox="0 0 210 297" xmlns="http://www.w3.org/2000/svg">
            <rect x="50" y="250" width="100" height="50" fill="red"/>
            <circle cx="130" cy="160" r="30" fill="green"/>
            <line x1="60" y1="120" x2="180" y2="100" stroke="blue" stroke-width="2"/>
            <text x="70" y="80" font-size="24">Hello, Piet!</text>
        </svg>
    "#;
    svg.to_string()
}