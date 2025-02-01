use std::str;
use std::task::Context;
use log::info;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_sys::{CanvasRenderingContext2d, Document, Element, DomParser, SupportedType, HtmlCanvasElement, MouseEvent, WheelEvent, Path2d};

#[derive(Clone)]
pub struct Point2D{
    pub x: f64,
    pub y: f64,
}
impl Point2D{
    pub fn new(x: f64, y: f64) -> Self {
        Point2D{x, y}
    }
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

pub struct Svg{
    location: Point2D,
    content: String,
}

impl Svg{
    pub fn new(location: Point2D, svg_text: &str) -> Self {
        Svg{location, content: svg_text.to_string()}
    }
}

impl Shape for Svg{
    fn color(&self) -> &str {
        "#0000ff"
    }

    fn line_width(&self) -> f64 {
        2.0
    }

    fn draw(&self, context: &CanvasRenderingContext2d){
        let path = Path2d::new().unwrap();

        let parser = DomParser::new().unwrap();
        let doc = parser.parse_from_string(&self.content, web_sys::SupportedType::ImageSvgXml).unwrap();

        if let Some(svg_element) = doc.query_selector("svg").ok().flatten() {
            web_sys::console::log_1(&"SVG 파싱 성공".into());
        } else {
            web_sys::console::log_1(&"⚠️ SVG 파싱 실패".into());
            return;
        }

        let paths = match doc.query_selector_all("path") { // SVG 요소에서 path 요소 가져오기
            Ok(paths) => paths,
            Err(_) => {
                info!("Failed to querySelector");
                return;
            }
        };

        if paths.length() == 0 {
            web_sys::console::log_1(&"⚠️ SVG 내부에 path 요소가 없음".into());
            return;
        }

        for i in 0..paths.length() {
            if let Some(path_element) = paths.item(i) {
                if let Ok(path_element) = path_element.dyn_into::<Element>() {
                    if let Some(d_attr) = path_element.get_attribute("d") {
                        path.add_path(&Path2d::new_with_path_string(&d_attr).unwrap());
                    }
                }
            }
        }

        context.save();
        context.set_fill_style(&JsValue::from_str("black"));
        context.translate(self.location.x, self.location.y).unwrap();
        context.fill_with_path_2d(&path);
        context.stroke_with_path(&path);
        context.restore();
    }
}