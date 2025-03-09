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
use kurbo::{Affine, Shape, Point, ParamCurve, ParamCurveNearest};

use crate::state::{State, ActionMode};
use super::geometry::{Point2D, Vector2D, BoundingRect2D};
use super::shape::{DrawShape, convert_to_color, DESIRED_ACCURACY};

#[derive(Debug, Clone)]
pub struct Rectangle{
    selected: bool,
    hovered: bool,
    color: String,
    line_width: f64,
    background: Option<String>,
    center: Point2D,
    width: f64,
    height: f64,
    rotation: f64,  // in radian,
    selected_control_point: i32,
}
impl Rectangle{
    pub fn new(start: Point2D, w: f64, h: f64, color: String, line_width: f64, background: Option<String>) -> Self {
        Rectangle{
            selected: false, 
            hovered: false, 
            color, 
            line_width, 
            background,
            center: Point2D::new(start.x + w * 0.5, start.y + h * 0.5), 
            width: w, 
            height: h, 
            rotation: 0.0,
            selected_control_point: -1}
    }

    fn control_points(&self, scale: f64) -> Vec<Point2D>{
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
            Point2D::new(self.center.x, self.center.y - self.height * 0.5 - 30.0 / scale)
            ];
        
        control_pts
    }

    fn center_point(&self) -> Point2D{
        let control_points = self.control_points(1.0);
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

impl DrawShape for Rectangle{
    fn color(&self) -> &str {
        &self.color
    }

    fn line_width(&self) -> f64 {
        self.line_width
    }

    fn min_point(&self) -> Point2D{
        Point2D::new(self.center.x - self.width * 0.5, self.center.y - self.height * 0.5)
    }

    fn max_point(&self) -> Point2D{
        Point2D::new(self.center.x + self.width * 0.5, self.center.y + self.height * 0.5)
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

    /// Given a shape and a point, returns the closest position on the shape's
    /// perimeter, or `None` if the shape is malformed.
    fn closest_perimeter_point(&self, pt: Point2D) -> Option<Point2D> {
        let mut best: Option<(kurbo::Point, f64)> = None;

        let rect = piet::kurbo::Rect::new(self.center.x - self.width * 0.5, self.center.y - self.height * 0.5,
            self.center.x + self.width * 0.5 , self.center.y + self.height * 0.5);

        let center = rect.center();
        let affine = Affine::translate((center.x, center.y))  // Move center to (0,0)
            * Affine::rotate(self.rotation)                         // Rotate
            * Affine::translate((-center.x, -center.y));                  // Move back

        let points = [
            rect.origin(),                       // Top-left
            Point::new(rect.x1, rect.y0),   // Top-right
            Point::new(rect.x1, rect.y1),   // Bottom-right
            Point::new(rect.x0, rect.y1),   // Bottom-left
        ];

        let transformed = points.map(|p| affine * p); // Apply the transformation

        let mut path = piet::kurbo::BezPath::new();
        if let Some(start) = transformed.first() {
            path.move_to(Point::new(start.x, start.y));

            for point in transformed.iter().skip(1) {
                path.line_to(Point::new(point.x, point.y));
            }
            path.close_path();

            for segment in path.path_segments(DESIRED_ACCURACY) {
                let nearest = segment.nearest(kurbo::Point::new(pt.x, pt.y), DESIRED_ACCURACY);
                if best.map(|(_, best_d)| nearest.distance_sq < best_d).unwrap_or(true) {
                    best = Some((segment.eval(nearest.t), nearest.distance_sq))
                }
            }
            return best.map(|(point, _)| Point2D::new(point.x, point.y));
        }

        None
    }

    /// Get the index of the control point that is hit by the mouse cursor.
    fn get_control_point(&self, x: f64, y: f64, scale: f64) -> i32{
        let mut control_pts = self.control_points(scale);
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
        let mut control_pts = self.control_points(1.0);
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

    fn draw(&self, context: &mut WebRenderContext, scale: f64){
        let _ = context.save();

        context.transform(Affine::translate((self.center.x, self.center.y)));
        context.transform(Affine::rotate(self.rotation));
        context.transform(Affine::translate((-self.center.x, -self.center.y)));

        let mut color = convert_to_color(&self.color);
        if self.hovered{
            color = Color::RED;
        }

        let adjusted_width = self.line_width / scale;
        let rect = piet::kurbo::Rect::new(self.center.x - self.width * 0.5, self.center.y - self.height * 0.5,
            self.center.x + self.width * 0.5 , self.center.y + self.height * 0.5);
        if let Some(ref background_color) = self.background {
            if background_color != "none"{
                context.fill(rect, &convert_to_color(background_color));
            }
        }
        context.stroke(rect, &color, adjusted_width);
        
        if self.selected{ self.draw_control_points(context, scale);}

        let _ = context.restore();
    }   

    fn draw_xor(&self, context: &mut WebRenderContext, state: &State){
        let _ = context.save();

        // 줌 및 팬 적용 (기존의 scale과 offset 유지)
        let scale = state.scale();
        let offset = state.offset();
        context.transform(Affine::new([scale, 0.0, 0.0, scale, offset.x, offset.y]));

        let adjusted_width = 1.0 / scale;

        self.draw(context, scale);
        if state.action_mode() == ActionMode::Drawing{
            if let Some(closest) = self.closest_perimeter_point(state.world_coord()){
                if state.world_coord().distance_to(closest) < 10.0{
                    // Define stroke style
                    let mut stroke_style = StrokeStyle::new();
                    stroke_style.set_line_cap(piet::LineCap::Round);
                    stroke_style.set_line_join(piet::LineJoin::Bevel);

                    // draw mark
                    let line = piet::kurbo::Line::new(
                        Point::new(closest.x - 5.0 / scale, closest.y - 5.0 / scale), 
                        Point::new(closest.x + 5.0 / scale, closest.y + 5.0 / scale));
                    context.stroke_styled(line, &Color::BLUE, adjusted_width, &stroke_style);

                    let line = piet::kurbo::Line::new(
                        Point::new(closest.x - 5.0 / scale, closest.y + 5.0 / scale), 
                        Point::new(closest.x + 5.0 / scale, closest.y - 5.0 / scale));
                    context.stroke_styled(line, &Color::BLUE, adjusted_width, &stroke_style);
                    //
                }
            }
        }

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
        let min_pt = self.min_point();
        let max_pt = self.max_point();
        let rect = piet::kurbo::Rect::new(min_pt.x, min_pt.y, max_pt.x, max_pt.y);
        context.stroke_styled(rect, &color, adjusted_width, &stroke_style);

        let adjusted_width = 5.0 / scale;
        let cirlce = piet::kurbo::Circle::new(piet::kurbo::Point::new(self.center.x, self.center.y), adjusted_width);
        context.fill(cirlce, &color);

    }

    // svg 텍스트를 반환한다.
    fn to_svg(&self, rect: BoundingRect2D) -> String{
        let origin = rect.min();
        let min = self.min_point();

        let mut style = "".to_string();
        if let Some(ref background) = self.background{
            style = format!(r#"fill:{background};stroke:{color}"#, 
            background = background, color = self.color);
        }
        else{
            style = format!(r#"fill:none;stroke:{color}"#, color = self.color);
        }

        format!(
                r#"<rect x="{x}" y="{y}" width="{width}" height="{height}" style="{style}"/>"#,
                x = min.x - origin.x,
                y = min.y - origin.y,
                width = self.width,
                height = self.height)

    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}