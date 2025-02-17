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

use web_sys::{CanvasRenderingContext2d};

use super::geometry::Vector2D;
use super::geometry::{Point2D};
use super::shape::{Shape};

#[derive(Debug, Clone)]
pub struct Rectangle{
    selected: bool,
    hovered: bool,
    color: String,
    line_width: f64,
    start: Point2D,
    width: f64,
    height: f64,
    rotation: f64,  // in radian
}
impl Rectangle{
    pub fn new(color: String, line_width: f64, start: Point2D, w: f64, h: f64) -> Self {
        Rectangle{selected: false, hovered: false, color, line_width, start: start, width: w, height: h, rotation: 0.0}
    }

    fn control_points(&self) -> Vec<Point2D>{
        let mut control_pts = vec![
            Point2D::new(self.start.x, self.start.y), 
            Point2D::new(self.start.x, self.start.y + self.height * 0.5),
            Point2D::new(self.start.x, self.start.y + self.height),
            Point2D::new(self.start.x + self.width * 0.5, self.start.y + self.height),
            Point2D::new(self.start.x + self.width, self.start.y + self.height),
            Point2D::new(self.start.x + self.width, self.start.y + self.height * 0.5),
            Point2D::new(self.start.x + self.width, self.start.y),
            Point2D::new(self.start.x + self.width * 0.5, self.start.y),
            Point2D::new(self.start.x + self.width * 0.5, self.start.y - 30.0)
            ];
        
        let center = Point2D::new(self.start.x + self.width * 0.5, self.start.y + self.height * 0.5);

        for pt in &mut control_pts{
            let mut dir = Vector2D::from_points(center, *pt);
            dir.rotate_by(self.rotation);
            pt.x = center.x + dir.x;
            pt.y = center.y + dir.y;
        }

        control_pts
    }

    fn center_point(&self) -> Point2D{
        let control_points = self.control_points();
        let start = control_points.get(0).unwrap();
        let end = control_points.get(4).unwrap();
        Point2D::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5)
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
        let control_pts = self.control_points();

        let adjusted_width = (5.0 / scale).powi(2);
        control_pts.iter().position(|p| (x - p.x).powi(2) + (y - p.y).powi(2) < adjusted_width).map_or(-1, |i| i as i32)
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
        self.start.x += dx;
        self.start.y += dy;
    }

    fn move_control_point_by(&mut self, index: i32, dx: f64, dy: f64) {
        let mut control_pts = self.control_points();
        let center = self.center_point();

        if index == 8{
            if let Some(pt) = control_pts.get_mut(index as usize) {
                let mut clone = pt.clone();
                clone.x += dx;
                clone.y += dy;

                let pt_dir = Vector2D::from_points(center, *pt);
                let clone_dir= Vector2D::from_points(center, clone);
                let angle = pt_dir.angle_to(clone_dir);
                self.rotation += angle;
            }
        }
        else{
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

            self.start.x = min.x;
            self.start.y = min.y;
            self.width = max.x - min.x;
            self.height = max.y - min.y;
        }
    }

    fn draw(&mut self, context: &CanvasRenderingContext2d, scale: f64){
        context.save();

        context.rotate(self.rotation).unwrap();
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

    // draw control points and rotation point
    fn draw_control_points(&self, context: &CanvasRenderingContext2d, scale: f64) {
        let adjusted_width = 5.0 / scale;

        context.save();

        let control_pts = self.control_points();
        context.set_fill_style(&"#29B6F2".into()); // color of control points
        for point in control_pts{
            context.fill_rect(point.x - adjusted_width, point.y - adjusted_width, adjusted_width * 2.0, adjusted_width * 2.0);
        }

        context.set_stroke_style(&"#29B6F2".into()); // color of line
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

        let adjusted_width = 5.0 / scale;
        let rotation_pt = Point2D::new((min_pt.x + max_pt.x) * 0.5, min_pt.y - 30.0 / scale);
        context.begin_path();
        context.arc(rotation_pt.x, rotation_pt.y, adjusted_width, 0.0, std::f64::consts::PI * 2.0).unwrap();
        context.fill();

        context.restore();
    }
}