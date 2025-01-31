use std::str;
use log::info;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, MouseEvent, WheelEvent};

#[derive(Clone)]
pub struct Point2D{
    pub x: f64,
    pub y: f64,
}

// Shape 트레이트 정의
pub trait Shape{
    fn color(&self) -> &str;
    fn line_width(&self) -> f64 { 2.0 }
    fn draw(&self, context: &CanvasRenderingContext2d);
}

pub struct Pencil{
    color: String,
    line_width: f64,
    points: Vec<Point2D>,
}
impl Pencil{
    pub fn new(color: String, line_width: f64, points: Vec<Point2D>) -> Self {
        Pencil{color, line_width, points}
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

    fn draw(&self, context: &CanvasRenderingContext2d){
        context.set_stroke_style(&JsValue::from_str(&self.color));
        context.set_line_width(self.line_width);
        context.begin_path();
        
        info!("draw pencil"); // 값을 콘솔에 출력
        if let Some(start) = self.points.first(){
            context.move_to(start.x, start.y);
            for point in self.points.iter().skip(1) {
                context.line_to(point.x, point.y);
            }
        }

        context.stroke();
    }   
}

pub struct Line{
    color: String,
    line_width: f64,
    start: Point2D,
    end: Point2D,
}
impl Line {
    pub fn new(color: String, line_width: f64, start: Point2D, end: Point2D) -> Self {
        Line {color, line_width, start, end}
    }
}

impl Shape for Line{
    fn color(&self) -> &str {
        &self.color
    }

    fn line_width(&self) -> f64 {
        self.line_width
    }

    fn draw(&self, context: &CanvasRenderingContext2d){
        context.set_stroke_style(&"#0000ff".into());
        context.set_line_width(self.line_width);
        context.begin_path();
        context.move_to(self.start.x, self.start.y);
        context.line_to(self.end.x, self.end.y);
        context.stroke();
    }   
}