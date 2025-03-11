use std::any::Any;
use std::collections::HashMap;
use std::f64::MAX;
use std::iter::Scan;
use std::str;
use std::sync::Arc;
use std::task::Context;
use std::thread::panicking;
use log::info;
use piet_web::WebRenderContext;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_sys::console::group;
use web_sys::console::info;
use web_sys::{window, CanvasRenderingContext2d};

use piet::{RenderContext, Color, StrokeStyle};
use kurbo::{Affine, Shape, Point, Vec2, ParamCurve, ParamCurveNearest};

use crate::start;
use crate::state::{State, ActionMode};
use super::geometry::{Point2D, Vector2D, BoundingRect2D};
use super::shape::{DrawShape, convert_to_color, DESIRED_ACCURACY};

#[derive(Debug, Clone)]
pub struct EllipticalArc{
    center: Point2D,
    radius_x: f64,
    radius_y: f64,
    rotation: f64,
    start_angle: f64,
    sweep_angle: f64,
    selected: bool,
    hovered: bool,
    color: String,
    background: Option<String>,
    line_width: f64,
    axis_x: Vector2D,
    axis_y: Vector2D,
    selected_control_point: i32
}
impl EllipticalArc{
    pub fn new(center: Point2D, rx: f64, ry: f64, rotation: f64, start_angle: f64, sweep_angle: f64, color: String, line_width: f64, background: Option<String>) -> Self {
        EllipticalArc{
            center, 
            radius_x: rx, 
            radius_y: ry, 
            rotation: rotation, 
            start_angle: start_angle, 
            sweep_angle,
            selected: false, 
            hovered: false, 
            color, 
            background,
            line_width , 
            axis_x: Vector2D::AXIS_X, 
            axis_y: Vector2D::AXIS_Y,
            selected_control_point: -1}
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
            Point2D::new(self.center.x, self.center.y - self.radius_y),
            Point2D::new(self.center.x, self.center.y - self.radius_y - 30.0)
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

    // 주어진 angle(라디안)에 해당하는 타원 위의 Point를 반환
    fn point_on_ellipse(&self, angle: f64) -> Point {
        let a = self.radius_x; // X축 반지름
        let b = self.radius_y; // Y축 반지름
        let center = self.center;
        let x_rotation = self.rotation;

        // 기본 타원 좌표 (회전 전)
        let x = a * angle.cos();
        let y = b * angle.sin();

        // 회전 변환 적용
        let rotated_x = x * x_rotation.cos() - y * x_rotation.sin();
        let rotated_y = x * x_rotation.sin() + y * x_rotation.cos();

        // 중심 좌표 적용
        Point {
            x: center.x + rotated_x,
            y: center.y + rotated_y,
        }
    }
}
impl DrawShape for EllipticalArc{
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

    fn bounding_rect(&self) -> super::geometry::BoundingRect2D {
        BoundingRect2D { min: self.min_point(), max: self.max_point() }
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

    /// Given a shape and a point, returns the closest position on the shape's
    /// perimeter, or `None` if the shape is malformed.
    fn closest_perimeter_point(&self, pt: Point2D) -> Option<Point2D> {
        let mut best: Option<(kurbo::Point, f64)> = None;

        let arc= piet::kurbo::Arc::new(
            Point::new(self.center.x, self.center.y), 
            Vec2::new(self.radius_x, self.radius_y),
            self.start_angle, self.sweep_angle,
           self.rotation 
        );

        for segment in arc.path_segments(DESIRED_ACCURACY) {
            let nearest = segment.nearest(kurbo::Point::new(pt.x, pt.y), DESIRED_ACCURACY);
            if best.map(|(_, best_d)| nearest.distance_sq < best_d).unwrap_or(true) {
                best = Some((segment.eval(nearest.t), nearest.distance_sq))
            }
        }
        best.map(|(point, _)| Point2D::new(point.x, point.y))
    }

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
        for pt in &mut control_pts{
            let mut dir = Vector2D::from_points(self.center, *pt);
            dir.rotate_by(self.rotation);
            pt.x = self.center.x + dir.x;
            pt.y = self.center.y + dir.y;
        }

        if index == 8{
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
                self.radius_x += dot * 0.5;
            } else if index == 3 || index == 7{
                let mut dir = Vector2D::from_points(self.center,*control_pts.get(index as usize).unwrap() );
                dir.normalize();
                let dot = dir.dot(Vector2D::new(dx, dy));
                self.center += dir * dot * 0.5;
                self.radius_y += dot * 0.5;
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
                self.radius_x = self.axis_x().dot(dir).abs();
                self.radius_y = self.axis_y().dot(dir).abs();
            }
        }
    }

    fn draw(&self, context: &mut WebRenderContext, scale: f64){
        let mut color = convert_to_color(&self.color);
        if self.hovered{
            color = Color::RED;
        }

        let adjusted_width = self.line_width / scale;

        let arc= piet::kurbo::Arc::new(
            Point::new(self.center.x, self.center.y), 
            Vec2::new(self.radius_x, self.radius_y),
            self.start_angle, self.sweep_angle,
           self.rotation 
        );
        if let Some(ref background_color) = self.background {
            if background_color != "none"{
                context.fill(arc, &convert_to_color(background_color));
            }
        }

        context.stroke(&arc, &color, adjusted_width);
        
        if self.selected{ self.draw_control_points(context, scale);}
    }   

    fn draw_xor(&self, context: &mut WebRenderContext, state: &State){
        let _ = context.save();

        // 줌 및 팬 적용 (기존의 scale과 offset 유지)
        let scale = state.scale();
        let offset = state.offset();
        context.transform(Affine::new([scale, 0.0, 0.0, scale, offset.x, offset.y]));

        self.draw(context, scale);
        if state.action_mode() == ActionMode::Drawing{
            let adjusted_width = 1.0 / scale;
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

    fn draw_control_points(&self, context: &mut WebRenderContext, scale: f64) {
        let adjusted_width = 5.0 / scale;

        let color = convert_to_color("#29B6F2");

        let control_pts = self.control_points();
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

    fn to_svg(&self, rect: BoundingRect2D) -> String{
        let origin = rect.min();

        let mut style = "".to_string();
        if let Some(ref background) = self.background{
            style = format!(r#"fill:{background};stroke:{color}"#, 
            background = background, color = self.color);
        }
        else{
            style = format!(r#"fill:none;stroke:{color}"#, color = self.color);
        }

        let mut svg = "<path d=".to_string();

        let start_point = self.point_on_ellipse(self.start_angle);
        let end_point = self.point_on_ellipse(self.start_angle + self.sweep_angle);

        let content = format!(r#""M {} {} A {} {}, {}, 1 1, {} {}""#, 
        start_point.x - origin.x, start_point.y - origin.y, 
        self.radius_x, self.radius_y, 
        self.rotation.to_degrees(),
        end_point.x - origin.x, end_point.y - origin.y).to_string();

        svg.push_str(&content);
        svg.push_str(format!(r#" style="{}"/>"#, style).as_str());

        svg
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}