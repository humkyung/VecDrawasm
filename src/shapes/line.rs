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
use web_sys::{window, CanvasRenderingContext2d, Element, DomParser, CanvasGradient, HtmlCanvasElement, Path2d, CssStyleDeclaration};

use super::geometry::Vector2D;
use super::geometry::{Point2D};
use super::shape::{Shape};

#[derive(Debug, Clone)]
pub struct Line{
    selected: bool,
    hovered: bool,
    color: String,
    line_width: f64,
    start: Point2D,
    end: Point2D,
}
impl Line {
    pub fn new(color: String, line_width: f64, start: Point2D, end: Point2D) -> Self {
        Line {selected: false, hovered: false, color, line_width, start, end}
    }
}

impl Shape for Line{
    fn color(&self) -> &str {
        &self.color
    }

    fn line_width(&self) -> f64 {
        self.line_width
    }

    fn is_hit(&self, x: f64, y: f64) -> bool {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        let d = dx * dx + dy * dy;
        let mut t = ((x - self.start.x) * dx + (y - self.start.y) * dy) / d;
        if t < 0.0 {
            t = 0.0;
        } else if t > 1.0 {
            t = 1.0;
        }

        let px = self.start.x + t * dx;
        let py = self.start.y + t * dy;
        let dx = px - x;
        let dy = py - y;
        dx * dx + dy * dy < 25.0
    }

    fn max_point(&self) -> Point2D{
        Point2D::new(self.start.x.max(self.end.x), self.start.y.max(self.end.y))
    }

    fn min_point(&self) -> Point2D{
        Point2D::new(self.start.x.min(self.end.x), self.start.y.min(self.end.y))
    }

    fn get_control_point(&self, x: f64, y: f64, scale: f64) -> i32{
        let adjusted_width = 1.0 / scale * 5.0;

        let dx = x - self.start.x;
        let dy = y - self.start.y;
        if dx * dx + dy * dy < adjusted_width * adjusted_width { return 0; }

        let dx = x - self.end.x;
        let dy = y - self.end.y;
        if dx * dx + dy * dy < adjusted_width * adjusted_width { return 1; }

        -1
    }

    fn is_selected(&self) -> bool {
        self.selected
    }

    fn set_selected(&mut self, selected: bool){
        self.selected = selected;
        info!("selected = {:?}", self.selected);
    }

    fn set_hovered(&mut self, value: bool) {
        self.hovered = value;
    }

    fn move_by(&mut self, dx: f64, dy: f64) {
        self.start.x += dx;
        self.start.y += dy;
        self.end.x += dx;
        self.end.y += dy;
    }

    fn move_control_point_by(&mut self, index: i32, dx: f64, dy: f64) {
        if index ==0{
            self.start.x += dx;
            self.start.y += dy;
        } else if index == 1{
            self.end.x += dx;
            self.end.y += dy;
        }
    }

    /*
        라인을 캔버스에 그린다.
     */
    fn draw(&mut self, context: &CanvasRenderingContext2d, scale: f64){
        if self.hovered{
            context.set_stroke_style(&JsValue::from_str("#ff0000"));
        }
        else{
            context.set_stroke_style(&JsValue::from_str(&self.color));
        }
        let adjusted_width = self.line_width / scale;
        context.set_line_width(adjusted_width);
        context.begin_path();
        context.move_to(self.start.x, self.start.y);
        context.line_to(self.end.x, self.end.y);
        context.close_path();
        context.stroke();

        if self.selected{ self.draw_control_points(context, scale);}
    }   

    fn draw_xor(&self, context: &CanvasRenderingContext2d, scale: f64){
        context.save();
        context.set_global_composite_operation("xor").expect("something goes wrong when apply xor");

        context.begin_path();
        context.move_to(self.start.x, self.start.y);
        context.line_to(self.end.x, self.end.y);
        context.close_path();

        context.set_stroke_style(&JsValue::from_str(&self.color));
        let adjusted_width = self.line_width / scale;
        context.set_line_width(adjusted_width);

        context.stroke();
        context.restore();
    }

    fn draw_control_points(&self, context: &CanvasRenderingContext2d, scale: f64) {
        context.set_fill_style(&"#FF0000".into()); // Red control points

        let adjusted_width = 1.0 / scale * 5.0;
        context.begin_path();
        context.rect(self.start.x - adjusted_width, self.start.y - adjusted_width, adjusted_width * 2.0, adjusted_width * 2.0);
        context.rect(self.end.x - adjusted_width, self.end.y - adjusted_width, adjusted_width * 2.0, adjusted_width * 2.0);
        context.fill();
    }
}