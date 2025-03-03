use std::any::Any;
use std::collections::HashMap;
use std::f64::MAX;
use std::iter::Scan;
use std::str;
use std::task::Context;
use std::thread::panicking;
use log::info;
use piet_web::WebRenderContext;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_sys::console::group;
use web_sys::console::info;
use web_sys::{window, CanvasRenderingContext2d, Element, DomParser, CanvasGradient, HtmlCanvasElement, Path2d, CssStyleDeclaration};

use piet::{RenderContext, Color, StrokeStyle, Text, TextLayout, TextLayoutBuilder, ImageFormat};
use kurbo::Affine;

use crate::state::State;
use super::geometry::{Point2D, Vector2D, BoundingRect2D};
use super::shape::hex_to_color;
use super::shape::Shape;

#[derive(Debug, Clone)]
pub struct Line{
    selected: bool,
    hovered: bool,
    color: String,
    line_width: f64,
    start: Point2D,
    end: Point2D,
    selected_control_point: i32,
}
impl Line {
    pub fn new(color: String, line_width: f64, start: Point2D, end: Point2D) -> Self {
        Line {
            selected: false, 
            hovered: false, 
            color, 
            line_width, 
            start, 
            end,
            selected_control_point: -1,}
    }
}

impl Shape for Line{
    fn color(&self) -> &str {
        &self.color
    }

    fn line_width(&self) -> f64 {
        self.line_width
    }

    fn is_hit(&self, x: f64, y: f64, scale: f64) -> bool {
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

    fn bounding_rect(&self) -> super::geometry::BoundingRect2D {
        BoundingRect2D { min: self.min_point(), max: self.max_point() }
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

    fn get_selected_control_point(&self) -> i32 {
        self.selected_control_point
    }

    fn set_selected_control_point(&mut self, index: i32) {
        self.selected_control_point = index;
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
    fn draw(&self, context: &mut WebRenderContext, scale: f64){
        let mut color = hex_to_color(&self.color);
        if self.hovered{
            color = Color::RED;
        }

        // Define stroke style
        let mut stroke_style = StrokeStyle::new();
        //stroke_style.set_dash_pattern([10.0 / scale, 5.0 / scale]); // Dashed line pattern
        stroke_style.set_line_cap(piet::LineCap::Round);
        stroke_style.set_line_join(piet::LineJoin::Bevel);

        let adjusted_width = self.line_width / scale;

        let line = piet::kurbo::Line::new((self.start.x, self.start.y), (self.end.x, self.end.y));
        context.stroke_styled(line, &color, adjusted_width, &stroke_style);

        if self.selected{ self.draw_control_points(context, scale);}
    }   

    // XOR로 그리기
    fn draw_xor(&self, context: &mut WebRenderContext, state: &State){
        let _ = context.save();

        // 줌 및 팬 적용 (기존의 scale과 offset 유지)
        let scale = state.scale();
        let offset = state.offset();
        info!("scale = {}, offset = {:?}", scale, offset);
        context.transform(Affine::new([scale, 0.0, 0.0, scale, offset.x, offset.y]));

        self.draw(context, scale);

        let _ = context.restore();
    }

    /// Draw control points
    fn draw_control_points(&self, context: &mut WebRenderContext, scale: f64) {
        let adjusted_width = 5.0 / scale;
        let rect = piet::kurbo::Rect::new(self.start.x - adjusted_width, self.start.y - adjusted_width, self.start.x + adjusted_width, self.start.y + adjusted_width);
        context.fill(rect, &Color::RED);
        let rect = piet::kurbo::Rect::new(self.end.x - adjusted_width, self.end.y - adjusted_width, self.end.x + adjusted_width, self.end.y + adjusted_width);
        context.fill(rect, &Color::RED);
    }

    fn to_svg(&self) -> String{
        "".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}