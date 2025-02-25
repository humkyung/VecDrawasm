use std::any::Any;
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

use super::geometry::Vector2D;
use super::geometry::{Point2D};
use super::shape::{Shape};

#[derive(Debug, Clone)]
pub struct TextBox{
    pub position: Point2D,
    pub text: String,
    rotation: f64,
    selected: bool,
    hovered: bool,
    color: String,
    axis_x: Vector2D,
    axis_y: Vector2D,
    pub width: f64,
    pub composition_text: String,
    pub cursor_position: usize,
}
impl TextBox{
    pub fn new(x: f64, y: f64) -> Self {
        TextBox{
            position: Point2D::new(x, y)
            , text: String::new()
            , rotation: 0.0
            , selected: false
            , hovered: false
            , color: "#000000".to_string()
            , axis_x: Vector2D::AXIS_X
            , axis_y: Vector2D::AXIS_Y,
            width: 50.0,
            composition_text: String::new(),
            cursor_position: 0,}
    }

    fn control_points(&self) -> Vec<Point2D>{
        let control_pts = vec![
            Point2D::new(self.position.x, self.position.y) ,
            Point2D::new(self.position.x, self.position.y - 30.0)
            ];

        control_pts
    }

    fn axis_x(&self) -> Vector2D{
        let mut axis_x = Vector2D::AXIS_X.clone();
        axis_x.rotate_by(self.rotation);
        axis_x
    }

    fn axis_y(&self) -> Vector2D{
        let mut axis_y = Vector2D::AXIS_Y.clone();
        axis_y.rotate_by(self.rotation);
        axis_y
    }

    pub fn contains(&self, mouse_x: f64, mouse_y: f64) -> bool {
        mouse_x >= self.position.x && mouse_x <= self.position.x + self.width && mouse_y >= self.position.y && mouse_y <= self.position.y + 30.0
    }

    pub fn get_char_index_at_cursor(&self) -> usize {
        self.cursor_position
    }

    pub fn get_byte_index_at_cursor(&self) -> usize {
        self.text
            .char_indices()
            .nth(self.cursor_position)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len())
    }

    pub fn insert_at_cursor(&mut self, value: &str, index: usize) {
        let byte_index = self.get_byte_index_at_cursor();
        self.text.insert_str(byte_index, value);
        self.cursor_position += value.chars().count();
    }

    pub fn delete_before_cursor(&mut self) {
        if self.cursor_position > 0 {
            let byte_index = self.get_byte_index_at_cursor();
            let prev_char_index = self
                .text
                .char_indices()
                .take(self.cursor_position)
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);

            self.text.replace_range(prev_char_index..byte_index, "");
            self.cursor_position -= 1;
        }
    }

    pub fn delete_at_cursor(&mut self) {
        if self.cursor_position < self.text.chars().count() {
            let start_index = self.get_byte_index_at_cursor();
            let end_index = self.text[start_index..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| start_index + i)
                .unwrap_or(self.text.len());

            self.text.replace_range(start_index..end_index, "");
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.text.chars().count() {
            self.cursor_position += 1;
        }
    }

    pub fn update_width(&mut self, text_width: f64) {
        // 최소 50px, 최대 400px
        self.width = text_width.clamp(50.0, 400.0) + 10.0; // padding 포함
    }
}
impl Shape for TextBox{
    fn color(&self) -> &str {
        &self.color
    }

    fn line_width(&self) -> f64 {
        0.0
    }

    fn max_point(&self) -> Point2D{
        Point2D::new(self.position.x, self.position.y)
    }

    fn min_point(&self) -> Point2D{
        Point2D::new(self.position.x, self.position.y)
    }

    fn is_hit(&self, x: f64, y: f64, scale: f64) -> bool {
        let index = self.get_control_point(x, y, scale);
        if index != -1{return true;}

        let min_pt = self.min_point();
        let max_pt = self.max_point();

        if x < min_pt.x || x > max_pt.x {return false;}
        if y < min_pt.y || y > max_pt.y {return false;}

        true
    }

    fn get_control_point(&self, x: f64, y: f64, scale: f64) -> i32{
        let mut control_pts = self.control_points();
        for pt in &mut control_pts{
            let mut dir = Vector2D::from_points(self.position, *pt);
            dir.rotate_by(self.rotation);
            pt.x = self.position.x + dir.x;
            pt.y = self.position.y + dir.y;
        }

        let adjusted_width = (10.0 / scale).powi(2);
        control_pts.iter().position(|p| (x - p.x).powi(2) + (y - p.y).powi(2) < adjusted_width).map_or(-1, |i| i as i32)
    }

    fn is_selected(&self) -> bool {
        self.selected
    }

    fn set_selected(&mut self, value: bool){
        self.selected = value;
    }

    fn set_hovered(&mut self, value: bool) {
        self.hovered = value;
    }

    fn move_by(&mut self, dx: f64, dy: f64) {
        self.position.x += dx;
        self.position.y += dy;
    }

    fn move_control_point_by(&mut self, index: i32, dx: f64, dy: f64) {
        let mut control_pts = self.control_points();
        for pt in &mut control_pts{
            let mut dir = Vector2D::from_points(self.position, *pt);
            dir.rotate_by(self.rotation);
            pt.x = self.position.x + dir.x;
            pt.y = self.position.y + dir.y;
        }

        if index == 0{
            self.position.x += dx;
            self.position.y += dy;
        }
    }

    fn draw(&mut self, context: &CanvasRenderingContext2d, scale: f64){
        context.save();

        context.translate(self.position.x, self.position.y).unwrap();
        context.rotate(self.rotation).unwrap();
        context.translate(-self.position.x, -self.position.y).unwrap();

        if self.hovered{
            context.set_stroke_style(&JsValue::from_str("#ff0000"));
        }
        else{
            context.set_stroke_style(&JsValue::from_str(&self.color));
        }

        context.set_fill_style(&"#000000".into()); // Black text
        context.set_font("20px Arial");
        context.fill_text(&self.text, self.position.x, self.position.y).unwrap();

        if self.selected{ self.draw_control_points(context, scale);}

        context.restore();
    }   

    fn draw_xor(&self, context: &CanvasRenderingContext2d, scale: f64){
        context.save();

        context.set_global_composite_operation("xor").unwrap();

        context.translate(self.position.x, self.position.y).unwrap();
        context.rotate(self.rotation).unwrap();
        context.translate(-self.position.x, -self.position.y).unwrap();

        if self.hovered{
            context.set_stroke_style(&JsValue::from_str("#ff0000"));
        }
        else{
            context.set_stroke_style(&JsValue::from_str(&self.color));
        }

        context.set_fill_style(&"#000000".into()); // Black text
        context.set_font("20px Arial");
        context.fill_text(&self.text, self.position.x, self.position.y).unwrap();

        context.restore();
    }

    fn draw_control_points(&self, context: &CanvasRenderingContext2d, scale: f64) {
        let adjusted_width = 5.0 / scale;

        context.save();

        let control_pts = self.control_points();
        context.set_fill_style(&"#29B6F2".into()); // Red control points
        for point in control_pts{
            context.fill_rect(point.x - adjusted_width, point.y - adjusted_width, adjusted_width * 2.0, adjusted_width * 2.0);
        }

        context.set_stroke_style(&"#29B6F2".into()); // blue line
        let adjusted_width = 0.5 / scale;
        context.set_line_width(adjusted_width);

        // ✅ Set dash pattern: [Dash length, Gap length]
        let dash_pattern = js_sys::Array::new();
        dash_pattern.push(&(adjusted_width * 3.0).into());  // dash
        dash_pattern.push(&(adjusted_width * 3.0).into());  // gap
        context.set_line_dash(&dash_pattern).unwrap();

        //context.begin_path();
        //context.rect(self.center.x - self.radius_x, self.center.y - self.radius_y, self.radius_x * 2.0, self.radius_y * 2.0);
        //context.stroke();

        context.restore();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}