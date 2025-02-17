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
pub struct Ellipse{
    center: Point2D,
    radius_x: f64,
    radius_y: f64,
    rotation: f64,
    start_angle: f64,
    end_angle: f64,
    selected: bool,
    hovered: bool,
    color: String,
    line_width: f64,
}
impl Ellipse{
    pub fn new(center: Point2D, rx: f64, ry: f64, rotation: f64, start_angle: f64, end_angle: f64, color: String, line_width: f64) -> Self {
        Ellipse{center, radius_x: rx, radius_y: ry, rotation: rotation, start_angle: start_angle, end_angle: end_angle, selected: false, hovered: false, color, line_width}
    }

    fn control_points(&self) -> Vec<Point2D>{
        let control_pts = vec![
            Point2D::new(self.center.x - self.radius_x, self.center.y - self.radius_y), 
            Point2D::new(self.center.x - self.radius_x, self.center.y),
            Point2D::new(self.center.x - self.radius_x, self.center.y + self.radius_y),
            Point2D::new(self.center.x, self.center.y + self.radius_y),
            Point2D::new(self.center.x + self.radius_x, self.center.y + self.radius_y),
            Point2D::new(self.center.x + self.radius_x, self.center.y),
            Point2D::new(self.center.x + self.radius_x, self.center.y - self.radius_y),
            Point2D::new(self.center.x, self.center.y - self.radius_y)
            ];

        control_pts
    }
}
impl Shape for Ellipse{
    fn color(&self) -> &str {
        &self.color
    }

    fn line_width(&self) -> f64 {
        self.line_width
    }

    fn max_point(&self) -> Point2D{
        Point2D::new(self.center.x + self.radius_x, self.center.y + self.radius_y)
    }

    fn min_point(&self) -> Point2D{
        Point2D::new(self.center.x - self.radius_x, self.center.y - self.radius_y)
    }

    fn is_hit(&self, x: f64, y: f64) -> bool {
        let min_pt = self.min_point();
        let max_pt = self.max_point();

        if x < min_pt.x || x > max_pt.x {return false;}
        if y < min_pt.y || y > max_pt.y {return false;}

        true
    }

    fn get_control_point(&self, x: f64, y: f64, scale: f64) -> i32{
        let control_pts = self.control_points();

        let adjusted_width = (5.0 / scale).powi(2);
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
        self.center.x += dx;
        self.center.y += dy;
    }

    fn move_control_point_by(&mut self, index: i32, dx: f64, dy: f64) {
        let mut control_pts = self.control_points();
        if index == 1 || index == 5{
            if let Some(pt) = control_pts.get_mut((index - 1) as usize){
                pt.x += dx;
            }
            if let Some(pt) = control_pts.get_mut(index as usize){
                pt.x += dx;
            }
            if let Some(pt) = control_pts.get_mut((index + 1) as usize){
                pt.x += dx;
            }
        } else if index == 3 || index == 7{
            if let Some(pt) = control_pts.get_mut((index - 1) as usize){
                pt.y += dy;
            }
            if let Some(pt) = control_pts.get_mut(index as usize){
                pt.y += dy;
            }
            
            let index = (index + 1) % control_pts.len() as i32;
            if let Some(pt) = control_pts.get_mut((index) as usize){
                pt.y += dy;
            }
        }
        else if index == 0 || index == 4{
            let mut at = index - 2;
            if at < 0 {at = (at + control_pts.len() as i32) % control_pts.len() as i32;}
            if let Some(pt) = control_pts.get_mut(at as usize){
                pt.y += dy;
            }

            at = index - 1;
            if at < 0 {at = (at + control_pts.len() as i32) % control_pts.len() as i32;}
            if let Some(pt) = control_pts.get_mut(at as usize){
                pt.y += dy;
            }

            if let Some(pt) = control_pts.get_mut(index as usize){
                pt.x += dx;
                pt.y += dy;
            }
            
            at = index + 1;
            at = at % control_pts.len() as i32;
            if let Some(pt) = control_pts.get_mut(at as usize){
                pt.x += dx;
            }

            at = index + 2;
            at = at % control_pts.len() as i32;
            if let Some(pt) = control_pts.get_mut(at as usize){
                pt.x += dx;
            }
        }
        else if index == 2 || index == 6{
            let mut at = index - 2;
            if at < 0 {at = (at + control_pts.len() as i32) % control_pts.len() as i32;}
            if let Some(pt) = control_pts.get_mut(at as usize){
                pt.x += dx;
            }

            at = index - 1;
            if at < 0 {at = (at + control_pts.len() as i32) % control_pts.len() as i32;}
            if let Some(pt) = control_pts.get_mut(at as usize){
                pt.x += dx;
            }

            if let Some(pt) = control_pts.get_mut(index as usize){
                pt.x += dx;
                pt.y += dy;
            }
            
            at = index + 1;
            at = at % control_pts.len() as i32;
            if let Some(pt) = control_pts.get_mut(at as usize){
                pt.y += dy;
            }

            at = index + 2;
            at = at % control_pts.len() as i32;
            if let Some(pt) = control_pts.get_mut(at as usize){
                pt.y += dy;
            }
        }

        let max = control_pts.iter().fold(Point2D::new(f64::MIN, f64::MIN), |acc, point| 
            Point2D::new(acc.x.max(point.x), acc.y.max(point.y))
        );

        let min = control_pts.iter().fold(Point2D::new(f64::MAX, f64::MAX), |acc, point| 
            Point2D::new(acc.x.min(point.x), acc.y.min(point.y))
        );

        self.center.x = (min.x + max.x) * 0.5;
        self.center.y = (min.y + max.y) * 0.5;
        self.radius_x = (max.x - min.x) * 0.5;
        self.radius_y = (max.y - min.y) * 0.5;
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
        context.ellipse(self.center.x, self.center.y, self.radius_x, self.radius_y, self.rotation, self.start_angle, self.end_angle);
        context.stroke();
        
        context.restore();

        if self.selected{ self.draw_control_points(context, scale);}
    }   

    fn draw_xor(&self, context: &CanvasRenderingContext2d, scale: f64){
        context.save();

        context.set_global_composite_operation("xor").unwrap();

        context.begin_path();
        context.ellipse(self.center.x, self.center.y, self.radius_x, self.radius_y, self.rotation, self.start_angle, self.end_angle);
        context.set_stroke_style(&JsValue::from_str(&self.color));
        let adjusted_width = self.line_width / scale;
        context.set_line_width(adjusted_width);
        context.stroke();

        context.restore();
    }

    fn draw_control_points(&self, context: &CanvasRenderingContext2d, scale: f64) {
        let adjusted_width = 1.0 / scale * 5.0;

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

        let min_pt = self.min_point();
        let max_pt = self.max_point();
        context.begin_path();
        context.move_to(min_pt.x, min_pt.y);
        context.line_to(max_pt.x, min_pt.y);
        context.line_to(max_pt.x, max_pt.y);
        context.line_to(min_pt.x, max_pt.y);
        context.line_to(min_pt.x, min_pt.y);
        context.stroke();

        context.restore();
    }
}