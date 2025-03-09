use std::any::Any;
use std::collections::HashMap;
use std::f64::MAX;
use std::iter::Scan;
use std::str;
use std::fmt::Debug;
use std::task::Context;
use std::thread::panicking;
use log::info;
use piet_web::WebRenderContext;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use piet::{RenderContext, Color, Text, TextLayout, TextLayoutBuilder, ImageFormat};
use kurbo::{Point};

use web_sys::console::group;
use web_sys::console::info;
use web_sys::{window, CanvasRenderingContext2d, Element, DomParser, CanvasGradient, HtmlCanvasElement, Path2d, CssStyleDeclaration};
use svgtypes::Transform;

use crate::state::State;
use super::geometry::{Point2D, Vector2D, BoundingRect2D};

pub const DESIRED_ACCURACY: f64 = 0.1;
// Shape 트레이트 정의
pub trait DrawShape : Debug + Send + Sync + Any{
    fn color(&self) -> &str;
    fn line_width(&self) -> f64 { 2.0 }
    fn max_point(&self) -> Point2D;
    fn min_point(&self) -> Point2D;
    fn bounding_rect(&self) -> BoundingRect2D;
    fn is_hit(&self, x: f64, y: f64, scale: f64) -> bool;
    fn closest_perimeter_point(&self, pt: Point2D) -> Option<Point2D>;
    fn get_control_point(&self, x: f64, y: f64, scale: f64) -> i32;
    fn get_selected_control_point(&self) -> i32;
    fn set_selected_control_point(&mut self, index: i32);
    fn is_selected(&self) -> bool;
    fn set_selected(&mut self, selected: bool);
    fn set_hovered(&mut self, hovered: bool);
    fn move_by(&mut self, dx: f64, dy: f64);
    fn move_control_point_by(&mut self, index: i32, dx: f64, dy: f64);
    fn draw(&self, context: &mut WebRenderContext, scale: f64);
    fn draw_xor(&self, context: &mut WebRenderContext, state: &State);
    fn draw_control_points(&self, context: &mut WebRenderContext, scale: f64);
    fn to_svg(&self, rect: BoundingRect2D) -> String;
    fn as_any(&self) -> &dyn Any;   // ✅ Needed for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// ✅ Implement PartialEq for dyn Shape (by type downcasting)
impl PartialEq for dyn DrawShape{
    fn eq(&self, other: &Self) -> bool {
        self.as_any().type_id() == other.as_any().type_id() // ✅ Compare types
    }
}

pub fn convert_to_color(color: &str) -> Color{
    if color.starts_with('#'){
        hex_to_color(color)
    }
    else{
        named_color_to_rgb(color)
    }
}

/// Convert hex color string to Color
pub fn hex_to_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        Color::rgb8(r, g, b)
    } else {
        Color::BLACK // Default fallback color
    }
}

fn named_color_to_rgb(color_name: &str) -> Color {
    let color_map: HashMap<&str, (u8, u8, u8)> = [
        ("black", (0, 0, 0)),
        ("white", (255, 255, 255)),
        ("red", (255, 0, 0)),
        ("green", (0, 128, 0)),
        ("blue", (0, 0, 255)),
        ("yellow", (255, 255, 0)),
        ("lightgray", (211, 211, 211)),
        ("gray", (128, 128, 128)),
        ("darkgray", (169, 169, 169))
    ].iter().cloned().collect();
    
    if let Some(&(r, g, b)) = color_map.get(color_name.to_lowercase().as_str()) {
        Color::rgb8(r, g, b)
    } else {
        Color::BLACK // 기본값
    }
}