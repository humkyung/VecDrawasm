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

use piet::{RenderContext, Color, Text, TextLayout, StrokeStyle};
use kurbo::Affine;

use web_sys::console::group;
use web_sys::console::info;
use web_sys::{window, CanvasRenderingContext2d, Element, DomParser, CanvasGradient, HtmlCanvasElement, Path2d, CssStyleDeclaration};
use svgtypes::Transform;

use crate::state::State;
use super::geometry::{Point2D, Vector2D};
use super::shape::{Shape, hex_to_color};

#[derive(Debug, Clone)]
pub struct Pencil{
    selected: bool,
    hovered: bool,
    color: String,
    line_width: f64,
    points: Vec<Point2D>,
    selected_control_point: i32,
}
impl Pencil{
    pub fn new(color: String, line_width: f64, points: Vec<Point2D>) -> Self {
        Pencil{
            selected: false, 
            hovered: false, 
            color, 
            line_width, 
            points,
            selected_control_point: -1,}
    }

    pub fn add_point(&mut self, point: Point2D){
        self.points.push(point);
    }
}
impl Shape for Pencil{
    fn color(&self) -> &str {
        &self.color
    }

    fn line_width(&self) -> f64 {
        self.line_width
    }

    fn max_point(&self) -> Point2D{
        self.points.iter().fold(Point2D::new(f64::MIN, f64::MIN), |acc, point| 
            Point2D::new(acc.x.max(point.x), acc.y.max(point.y))
        )
    }

    fn min_point(&self) -> Point2D{
        self.points.iter().fold(Point2D::new(f64::MAX, f64::MAX), |acc, point| 
            Point2D::new(acc.x.min(point.x), acc.y.min(point.y))
        )
    }

    fn is_hit(&self, x: f64, y: f64, scale: f64) -> bool {
        let adjusted_width = (5.0 / scale).powf(2.0);

        for point in self.points.iter() {
            let dx = x - point.x;
            let dy = y - point.y;
            if dx * dx + dy * dy < adjusted_width {
                return true;
            }
        }
        false
    }

    fn get_control_point(&self, x: f64, y: f64, scale: f64) -> i32{
        let adjusted_width = (5.0 / scale).powf(2.0);

        for (index, point) in self.points.iter().enumerate() {
            let dx = x - point.x;
            let dy = y - point.y;
            if dx * dx + dy * dy < adjusted_width * adjusted_width {
                return index as i32;
            }
        }
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
    }

    fn set_hovered(&mut self, value: bool) {
        self.hovered = value;
    }

    fn move_by(&mut self, dx: f64, dy: f64) {
        for point in self.points.iter_mut(){
            point.x += dx;
            point.y += dy;
        }
    }

    fn move_control_point_by(&mut self, index: i32, dx: f64, dy: f64) {
        if index >=0 && index < self.points.len() as i32{
            if let Some(point) = self.points.get_mut(index as usize){
                point.x += dx;
                point.y += dy;
            }
        }
    }

    fn draw(&self, context: &mut WebRenderContext, scale: f64){
        let mut color = hex_to_color(&self.color);
        if self.hovered{
            color = Color::RED;
        }

        let adjusted_width = self.line_width / scale;
        if let Some(mut start) = self.points.first() {
            for point in self.points.iter().skip(1) {
                let line = piet::kurbo::Line::new((start.x, start.y), (point.x, point.y));
                context.stroke(line, &color, adjusted_width);
                start = point;
            }
        }

        if self.selected{ self.draw_control_points(context, scale);}
    }   

    fn draw_xor(&self, context: &mut WebRenderContext, state: &State){
        context.save();

        // 줌 및 팬 적용 (기존의 scale과 offset 유지)
        let scale = state.scale();
        let offset = state.offset();
        context.transform(Affine::new([scale, 0.0, 0.0, scale, offset.x, offset.y]));

        self.draw(context, scale);

        context.restore();
    }

    fn draw_control_points(&self, context: &mut WebRenderContext, scale: f64) {
        let adjusted_width = 5.0 / scale;

        for point in self.points.clone(){
            let rect = piet::kurbo::Rect::new(point.x - adjusted_width, point.y - adjusted_width, point.x + adjusted_width,point.y + adjusted_width);
            context.fill(rect, &Color::RED);
        }

        // Define stroke style
        let mut stroke_style = StrokeStyle::new();
        stroke_style.set_dash_pattern([3.0 / scale, 3.0 / scale]); // Dashed line pattern
        stroke_style.set_line_cap(piet::LineCap::Round);
        stroke_style.set_line_join(piet::LineJoin::Bevel);

        let adjusted_width = 0.5 / scale;
        let min_pt = self.min_point();
        let max_pt = self.max_point();
        let rect = piet::kurbo::Rect::new(min_pt.x, min_pt.y, max_pt.x, max_pt.y);
        context.stroke_styled(rect, &Color::RED, adjusted_width, &stroke_style);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}