use std::any::Any;
use std::collections::HashMap;
use std::f64::consts::PI;
use std::f64::MAX;
use std::iter::Scan;
use std::str;
use std::task::Context;
use std::thread::panicking;
use js_sys::Intl::get_canonical_locales;
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
    center: Point2D,
    width: f64,
    height: f64,
    rotation: f64,  // in radian,
    selected_control_point: i32,
}
impl Rectangle{
    pub fn new(color: String, line_width: f64, start: Point2D, w: f64, h: f64) -> Self {
        Rectangle{
            selected: false, 
            hovered: false, 
            color, 
            line_width, 
            center: Point2D::new(start.x + w * 0.5, start.y + h * 0.5), 
            width: w, 
            height: h, 
            rotation: 0.0,
            selected_control_point: -1}
    }

    fn control_points(&self) -> Vec<Point2D>{
        let control_pts = vec![
            Point2D::new(self.center.x - self.width * 0.5, self.center.y - self.height * 0.5), 
            Point2D::new(self.center.x - self.width * 0.5, self.center.y),
            Point2D::new(self.center.x - self.width * 0.5, self.center.y + self.height * 0.5),
            Point2D::new(self.center.x, self.center.y + self.height * 0.5),
            Point2D::new(self.center.x + self.width * 0.5, self.center.y + self.height * 0.5),
            Point2D::new(self.center.x + self.width * 0.5, self.center.y),
            Point2D::new(self.center.x + self.width * 0.5, self.center.y - self.height * 0.5),
            Point2D::new(self.center.x, self.center.y - self.height * 0.5),
            Point2D::new(self.center.x, self.center.y),
            Point2D::new(self.center.x, self.center.y - self.height * 0.5 - 30.0)
            ];
        
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
        Point2D::new(self.center.x + self.width * 0.5, self.center.y + self.height * 0.5)
    }

    fn min_point(&self) -> Point2D{
        Point2D::new(self.center.x - self.width * 0.5, self.center.y - self.height * 0.5)
    }

    fn is_hit(&self, x: f64, y: f64, scale: f64) -> bool {
        let index = self.get_control_point(x, y, scale);
        if index != -1{return true;}

        let min_pt = self.min_point();
        let max_pt = self.max_point();
        if x < min_pt.x {return false;}
        if x > max_pt.x {return false;}
        if y < min_pt.y {return false;}
        if y > max_pt.y {return false;}

        true
    }

    /// Get the index of the control point that is hit by the mouse cursor.
    fn get_control_point(&self, x: f64, y: f64, scale: f64) -> i32{
        let mut control_pts = self.control_points();
        for pt in &mut control_pts{
            let mut dir = Vector2D::from_points(self.center, *pt);
            dir.rotate_by(self.rotation);
            pt.x = self.center.x + dir.x;
            pt.y = self.center.y + dir.y;
        }

        let adjusted_width = (10.0 / scale).powi(2);
        control_pts.iter().position(|p| (x - p.x).powi(2) + (y - p.y).powi(2) < adjusted_width).map_or(-1, |i| i as i32)
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
        self.center.x += dx;
        self.center.y += dy;
    }

    fn move_control_point_by(&mut self, index: i32, dx: f64, dy: f64) {
        let mut control_pts = self.control_points();
        for pt in &mut control_pts{
            let mut dir = Vector2D::from_points(self.center, *pt);
            dir.rotate_by(self.rotation);
            pt.x = self.center.x + dir.x;
            pt.y = self.center.y + dir.y;
        }

        if index == 8{
            self.center.x += dx;
            self.center.y += dy;
        }
        else if index == 9{
            if let Some(pt) = control_pts.get_mut(index as usize) {
                let mut clone = pt.clone();
                clone.x += dx;
                clone.y += dy;

                let pt_dir = Vector2D::from_points(self.center, *pt);
                let clone_dir= Vector2D::from_points(self.center, clone);
                let angle = pt_dir.angle_to(clone_dir);
                self.rotation += angle;
            }
        }
        else{
            if index == 1 || index == 5{
                let mut dir = Vector2D::from_points(self.center,*control_pts.get(index as usize).unwrap() );
                dir.normalize();
                let dot = dir.dot(Vector2D::new(dx, dy));
                self.center += dir * dot * 0.5;
                self.width += dot;
            } else if index == 3 || index == 7{
                let mut dir = Vector2D::from_points(self.center,*control_pts.get(index as usize).unwrap() );
                dir.normalize();
                let dot = dir.dot(Vector2D::new(dx, dy));
                self.center += dir * dot * 0.5;
                self.height += dot;
            }
            else if index == 0 || index == 2 || index == 4 || index == 6{
                let mut pt = *control_pts.get(index as usize).unwrap();
                pt.x += dx;
                pt.y += dy;
                if index == 0{
                    let opposite = *control_pts.get(4).unwrap();
                    self.center.x = (pt.x + opposite.x) * 0.5;
                    self.center.y = (pt.y + opposite.y) * 0.5;
                }
                else if index == 2{
                    let opposite = *control_pts.get(6).unwrap();
                    self.center.x = (pt.x + opposite.x) * 0.5;
                    self.center.y = (pt.y + opposite.y) * 0.5;
                }
                else if index == 4{
                    let opposite = *control_pts.get(0).unwrap();
                    self.center.x = (pt.x + opposite.x) * 0.5;
                    self.center.y = (pt.y + opposite.y) * 0.5;
                }
                else if index == 6{
                    let opposite = *control_pts.get(2).unwrap();
                    self.center.x = (pt.x + opposite.x) * 0.5;
                    self.center.y = (pt.y + opposite.y) * 0.5;
                }

                let dir = Vector2D::from_points(self.center, pt);
                self.width = self.axis_x().dot(dir).abs() * 2.0;
                self.height = self.axis_y().dot(dir).abs() * 2.0;
            }
        }
    }

    fn draw(&mut self, context: &CanvasRenderingContext2d, scale: f64){
        context.save();

        context.translate(self.center.x, self.center.y).unwrap();
        context.rotate(self.rotation).unwrap();
        context.translate(-self.center.x, -self.center.y).unwrap();

        if self.hovered{
            context.set_stroke_style(&JsValue::from_str("#ff0000"));
        }
        else{
            context.set_stroke_style(&JsValue::from_str(&self.color));
        }
        let adjusted_width = self.line_width / scale;
        context.set_line_width(adjusted_width);
        context.begin_path();
        context.rect(self.center.x - self.width * 0.5, self.center.y - self.height * 0.5, self.width, self.height);
        context.stroke();
        
        if self.selected{ self.draw_control_points(context, scale);}

        context.restore();
    }   

    fn draw_xor(&self, context: &CanvasRenderingContext2d, scale: f64){
        context.save();

        context.set_global_composite_operation("xor").unwrap();

        context.begin_path();
        context.rect(self.center.x - self.width * 0.5, self.center.y - self.height * 0.5, self.width, self.height);

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

        context.begin_path();
        context.rect(self.center.x - self.width * 0.5, self.center.y - self.height * 0.5, self.width, self.height);
        context.stroke();

        let adjusted_width = 5.0 / scale;
        context.begin_path();
        context.arc(self.center.x, self.center.y, adjusted_width, 0.0, std::f64::consts::PI * 2.0).unwrap();
        context.fill();

        context.restore();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}