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

use piet::{RenderContext, Color, StrokeStyle};
use kurbo::{Affine, Point, Shape, ParamCurve, ParamCurveNearest};

use web_sys::console::group;
use web_sys::console::info;
use web_sys::{window, CanvasRenderingContext2d, Element, DomParser, CanvasGradient, HtmlCanvasElement, Path2d, CssStyleDeclaration};
use svgtypes::Transform;

use crate::state::{State, ActionMode};
use super::geometry::{Point2D, Vector2D, BoundingRect2D};
use super::shape::{DrawShape, hex_to_color, convert_to_color, DESIRED_ACCURACY};

#[derive(Debug, Clone)]
pub struct Polyline{
    selected: bool,
    hovered: bool,
    color: String,
    background: Option<String>,
    line_width: f64,
    points: Vec<Point2D>,
    selected_control_point: i32,
}
impl Polyline{
    pub fn new(points: Vec<Point2D>, color: String, line_width: f64, background: Option<String>) -> Self {
        Polyline{
            selected: false, 
            hovered: false, 
            color, 
            background,
            line_width, 
            points,
            selected_control_point: -1,}
    }

    pub fn add_point(&mut self, point: Point2D){
        self.points.push(point);
    }
}
impl DrawShape for Polyline{
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

    fn bounding_rect(&self) -> super::geometry::BoundingRect2D {
        BoundingRect2D { min: self.min_point(), max: self.max_point() }
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

    /// Given a shape and a point, returns the closest position on the shape's
    /// perimeter, or `None` if the shape is malformed.
    fn closest_perimeter_point(&self, pt: Point2D) -> Option<Point2D> {
        let mut best: Option<(kurbo::Point, f64)> = None;

        let mut path = piet::kurbo::BezPath::new();
        if let Some(start) = self.points.first() {
            path.move_to(Point::new(start.x, start.y));

            for point in self.points.iter().skip(1) {
                path.line_to(Point::new(point.x, point.y));
            }
            if let Some(ref background) = self.background{
                path.close_path();
            }
        }

        for segment in path.path_segments(DESIRED_ACCURACY) {
            let nearest = segment.nearest(kurbo::Point::new(pt.x, pt.y), DESIRED_ACCURACY);
            if best.map(|(_, best_d)| nearest.distance_sq < best_d).unwrap_or(true) {
                best = Some((segment.eval(nearest.t), nearest.distance_sq))
            }
        }
        best.map(|(point, _)| Point2D::new(point.x, point.y))
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

    // 다중 직선을 그린다.
    fn draw(&self, context: &mut WebRenderContext, scale: f64){
        let mut color = hex_to_color(&self.color);
        if self.hovered{
            color = Color::RED;
        }

        let mut stroke_style = StrokeStyle::new();
        stroke_style.set_line_cap(piet::LineCap::Round);
        stroke_style.set_line_join(piet::LineJoin::Bevel);

        let adjusted_width = self.line_width / scale;
        if let Some(start) = self.points.first() {
            let mut path = piet::kurbo::BezPath::new();
            path.move_to(Point::new(start.x, start.y));

            for point in self.points.iter().skip(1) {
                path.line_to(Point::new(point.x, point.y));
            }
            if let Some(ref background) = self.background{
                path.close_path();

                context.fill(path.clone(), &convert_to_color(&background));
                context.stroke_styled(path, &color, adjusted_width, &stroke_style);
            }
            else{
                context.stroke_styled(path, &color, adjusted_width, &stroke_style);
            }
        }

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

    // svg 문자열을 반환한다.
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

        let mut svg = "<polyline points=\"".to_string();
        let point_strings: Vec<String> = self.points.iter().map(|pt| (*pt - origin).to_string()).collect();
        svg.push_str(&point_strings.join(" "));
        svg.push_str("\" ");
        svg.push_str(format!(r#"style="{}"/>"#, style).as_str());

        svg
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}