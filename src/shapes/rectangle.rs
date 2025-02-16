use std::collections::HashMap;
use std::f64::MAX;
use std::iter::Scan;
use std::str;
use std::task::Context;
use std::thread::panicking;
use log::info;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_sys::console::group;
use web_sys::console::info;
use web_sys::{window, CanvasRenderingContext2d};

use super::shape::{Point2D, Shape};

#[derive(Debug, Clone)]
pub struct Rectangle{
    selected: bool,
    hovered: bool,
    color: String,
    line_width: f64,
    start: Point2D,
    width: f64,
    height: f64
}
impl Rectangle{
    pub fn new(color: String, line_width: f64, start: Point2D, w: f64, h: f64) -> Self {
        Rectangle{selected: false, hovered: false, color, line_width, start: start, width: w, height: h}
    }
}
impl Shape for Rectangle{
    fn color(&self) -> &str {
        &self.color
    }

    fn line_width(&self) -> f64 {
        self.line_width
    }

    fn max_point(&self) -> Point2D{
        Point2D::new(self.start.x + self.width, self.start.y + self.height)
    }

    fn min_point(&self) -> Point2D{
        self.start.clone()
    }

    fn is_hit(&self, x: f64, y: f64) -> bool {
        if x < self.start.x {return false;}
        if x > self.start.x + self.width {return false;}
        if y < self.start.y {return false;}
        if y > self.start.y + self.height{return false;}

        true
    }

    fn get_control_point(&self, x: f64, y: f64, scale: f64) -> i32{
        -1
    }

    fn is_selected(&self) -> bool {
        self.selected
    }

    fn set_selected(&mut self, selected: bool){
        self.selected = selected;
    }

    fn set_hovered(&mut self, value: bool) {
        self.hovered = value;
    }

    fn move_by(&mut self, dx: f64, dy: f64) {
    }

    fn move_control_point_by(&mut self, index: i32, dx: f64, dy: f64) {
    }

    fn draw(&mut self, context: &CanvasRenderingContext2d, scale: f64){
        context.save();

        if self.hovered{
            context.set_stroke_style(&JsValue::from_str("#ff0000"));
        }
        else{
            context.set_stroke_style(&JsValue::from_str(&self.color));
        }
        let adjusted_width = self.line_width / scale;
        context.set_line_width(adjusted_width);
        context.begin_path();

        context.rect(self.start.x, self.start.y, self.width, self.height);
        context.stroke();
        
        context.restore();

        if self.selected{ self.draw_control_points(context, scale);}
    }   

    fn draw_xor(&self, context: &CanvasRenderingContext2d, scale: f64){
        context.save();

        context.set_global_composite_operation("xor").unwrap();

        context.begin_path();
        context.rect(self.start.x, self.start.y, self.width, self.height);

        context.set_stroke_style(&JsValue::from_str(&self.color));
        let adjusted_width = self.line_width / scale;
        context.set_line_width(adjusted_width);
        context.stroke();

        context.restore();
    }

    fn draw_control_points(&self, context: &CanvasRenderingContext2d, scale: f64) {
        let adjusted_width = 1.0 / scale * 5.0;

        context.save();
        context.restore();
    }
}