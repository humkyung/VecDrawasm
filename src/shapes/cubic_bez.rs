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
use piet_web::WebRenderContext;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use piet::{RenderContext, Color, StrokeStyle};
use kurbo::{Affine, CubicBez};

use crate::state::State;
use super::geometry::{Point2D, Vector2D, BoundingRect2D};
use super::shape::{Shape, convert_to_color};

#[derive(Debug, Clone)]
pub struct CubicBezier{
    selected: bool,
    hovered: bool,
    color: String,
    line_width: f64,
    background: Option<String>,
    p0: Point2D,
    p1: Point2D,
    p2: Point2D,
    p3: Point2D,
    rotation: f64,  // in radian,
    selected_control_point: i32,
}
impl CubicBezier{
    pub fn new(p0: Point2D, p1: Point2D, p2: Point2D, p3: Point2D, color: String, line_width: f64, background: Option<String>) -> Self {
        CubicBezier{
            selected: false, 
            hovered: false, 
            color, 
            line_width, 
            background,
            p0,
            p1,
            p2,
            p3,
            rotation: 0.0,
            selected_control_point: -1}
    }

    fn control_points(&self, scale: f64) -> Vec<Point2D>{
        let control_pts = vec![
            Point2D::new(self.p0.x, self.p0.y), 
            Point2D::new(self.p1.x, self.p1.y),
            Point2D::new(self.p2.x, self.p2.y),
            Point2D::new(self.p3.x, self.p3.y)
            ];
        
        control_pts
    }

    fn center_point(&self) -> Point2D{
        let pts = self.control_points(1.0);

        let mut x: f64 = 0.0;
        let mut y: f64 = 0.0;
        pts.iter().for_each(|pt|{
            x += pt.x;
            y += pt.y;
        });
        x /= pts.len() as f64;
        y /= pts.len() as f64;

        Point2D::new(x, y)
    }

    /*
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
    */
}

impl Shape for CubicBezier {
    fn color(&self) -> &str {
        &self.color
    }

    fn line_width(&self) -> f64 {
        self.line_width
    }

    fn min_point(&self) -> Point2D{
        let pts = self.control_points(1.0);
        
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MAX;
        let mut max_y = f64::MAX;
        pts.iter().for_each(|pt|{
            if min_x == f64::MAX{
                min_x = pt.x;
            } else if min_x > pt.x{
                min_x = pt.x;
            }

            if min_y == f64::MAX{
                min_y = pt.y;
            }else if min_y > pt.y{
                min_y = pt.y
            }

            if max_x == f64::MAX{
                max_x = pt.x;
            }else if max_x < pt.x{
                max_x = pt.x;
            }

            if max_y == f64::MAX{
                max_y = pt.y;
            }else if max_y < pt.y{
                max_y = pt.y;
            }
        });

        Point2D::new(min_x, min_y)
    }

    fn max_point(&self) -> Point2D{
        let pts = self.control_points(1.0);
        
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MAX;
        let mut max_y = f64::MAX;
        pts.iter().for_each(|pt|{
            if min_x == f64::MAX{
                min_x = pt.x;
            } else if min_x > pt.x{
                min_x = pt.x;
            }

            if min_y == f64::MAX{
                min_y = pt.y;
            }else if min_y > pt.y{
                min_y = pt.y
            }

            if max_x == f64::MAX{
                max_x = pt.x;
            }else if max_x < pt.x{
                max_x = pt.x;
            }

            if max_y == f64::MAX{
                max_y = pt.y;
            }else if max_y < pt.y{
                max_y = pt.y;
            }
        });

        Point2D::new(max_x, max_y)
    }

    fn bounding_rect(&self) -> super::geometry::BoundingRect2D {
        BoundingRect2D { min: self.min_point(), max: self.max_point() }
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
        let mut control_pts = self.control_points(scale);
        for pt in &mut control_pts{
            pt.x += x;
            pt.y += y;
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
        self.p0.x += dx;
        self.p0.y += dy;
        self.p1.x += dx;
        self.p1.y += dy;
        self.p2.x += dx;
        self.p2.y += dy;
        self.p3.x += dx;
        self.p3.y += dy;
    }

    fn move_control_point_by(&mut self, index: i32, dx: f64, dy: f64) {
        if index == 0{
            self.p0.x += dx;
            self.p0.y += dy;
        }
        else if index == 1{
            self.p1.x += dx;
            self.p1.y += dy;
        }else if index == 2{
            self.p2.x += dx;
            self.p2.y += dy;
        }else if index == 3{
            self.p3.x += dx;
            self.p3.y += dy;
        }
    }

    fn draw(&self, context: &mut WebRenderContext, scale: f64){
        let _ = context.save();

        let mut color = convert_to_color(&self.color);
        if self.hovered{
            color = Color::RED;
        }

        let adjusted_width = self.line_width / scale;
        let bezier = piet::kurbo::CubicBez::new(kurbo::Point::new(self.p0.x, self.p0.y), kurbo::Point::new(self.p1.x, self.p1.y), 
        kurbo::Point::new(self.p2.x, self.p2.y), kurbo::Point::new(self.p3.x, self.p3.y));
        if let Some(ref background_color) = self.background {
            context.fill(bezier, &convert_to_color(background_color));
        }
        context.stroke(bezier, &color, adjusted_width);
        
        if self.selected{ self.draw_control_points(context, scale);}

        let _ = context.restore();
    }   

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

    // draw control points and rotation point
    fn draw_control_points(&self, context: &mut WebRenderContext, scale: f64) {
        let adjusted_width = 5.0 / scale;

        let color = convert_to_color("#29B6F2");

        let control_pts = self.control_points(scale);
        for point in control_pts{
            let rect = piet::kurbo::Rect::new(point.x - adjusted_width, point.y - adjusted_width,
                point.x + adjusted_width, point.y + adjusted_width);
            context.fill(rect, &color);
        }

        // Define stroke style
        let mut stroke_style = StrokeStyle::new();
        stroke_style.set_dash_pattern([3.0 / scale, 3.0 / scale]); // Dashed line pattern
        stroke_style.set_line_cap(piet::LineCap::Round);
        stroke_style.set_line_join(piet::LineJoin::Bevel);

        let adjusted_width = 0.5 / scale;

        let mut path = piet::kurbo::BezPath::new();
        path.move_to(kurbo::Point::new(self.p0.x, self.p0.y));
        path.line_to(kurbo::Point::new(self.p1.x, self.p1.y));
        path.line_to(kurbo::Point::new(self.p2.x, self.p2.y));
        path.line_to(kurbo::Point::new(self.p3.x, self.p3.y));
        context.stroke_styled(path, &color, adjusted_width, &stroke_style);
    }

    // svg 텍스트를 반환한다.
    fn to_svg(&self, rect: BoundingRect2D) -> String{
        "".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}