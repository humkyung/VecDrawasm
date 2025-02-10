use std::collections::HashMap;
use std::iter::Scan;
use std::str;
use std::task::Context;
use log::info;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_sys::console::group;
use web_sys::console::info;
use web_sys::{window, CanvasRenderingContext2d, Element, DomParser, CanvasGradient, HtmlCanvasElement, Path2d, CssStyleDeclaration};
use svgtypes::Transform;

#[derive(Debug, Clone, Copy)]
pub struct Point2D{
    pub x: f64,
    pub y: f64,
}
impl Point2D{
    pub fn new(x: f64, y: f64) -> Self {
        Point2D{x, y}
    }

    pub fn set_x(&mut self, value: f64){
        self.x = value;
    } 

    pub fn set_y(&mut self, value: f64){
        self.y = value;
    }
}

// Shape 트레이트 정의
pub trait Shape{
    fn color(&self) -> &str;
    fn line_width(&self) -> f64 { 2.0 }
    fn is_hit(&self, x: f64, y: f64) -> bool;
    fn set_hovered(&mut self, hovered: bool);
    fn draw(&mut self, context: &CanvasRenderingContext2d, scale: f64);
    fn draw_xor(&self, context: &CanvasRenderingContext2d);
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

    fn is_hit(&self, x: f64, y: f64) -> bool {
        for point in self.points.iter() {
            let dx = x - point.x;
            let dy = y - point.y;
            if dx * dx + dy * dy < 25.0 {
                return true;
            }
        }
        false        
    }

    fn set_hovered(&mut self, hovered: bool) {
        if hovered {
            self.color = "#ff0000".to_string();
        } else {
            self.color = "#000000".to_string();
        }
    }

    fn draw(&mut self, context: &CanvasRenderingContext2d, scale: f64){
        context.set_stroke_style(&JsValue::from_str(&self.color));
        let adjusted_width = self.line_width / scale;
        context.set_line_width(adjusted_width);
        context.begin_path();
        
        if let Some(start) = self.points.first(){
            context.move_to(start.x, start.y);
            for point in self.points.iter().skip(1) {
                context.line_to(point.x, point.y);
            }
            context.stroke();
        }
    }   

    fn draw_xor(&self, context: &CanvasRenderingContext2d){
        if let Some(start) = self.points.first(){
            context.set_global_composite_operation("xor").unwrap();

            context.begin_path();
            context.move_to(start.x, start.y);
            for point in self.points.iter().skip(1) {
                context.line_to(point.x, point.y);
            }

            context.set_stroke_style(&JsValue::from_str(&self.color));
            context.set_line_width(self.line_width);

            context.stroke();

            context.set_global_composite_operation("source-over").unwrap(); // 기본 모드로 복원
        }
    }
}

#[derive(Debug, Clone)]
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

    fn is_hit(&self, x: f64, y: f64) -> bool {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        let d = dx * dx + dy * dy;
        let mut t = ((x - self.start.x) * dx + (y - self.start.y) * dy) / d;
        if t < 0.0 {
            t = 0.0;
        } else if t > 1.0 {
            t = 1.0;
        }

        let px = self.start.x + t * dx;
        let py = self.start.y + t * dy;
        let dx = px - x;
        let dy = py - y;
        dx * dx + dy * dy < 25.0
    }

    fn set_hovered(&mut self, hovered: bool) {
        /*
        if hovered {
            self.color = "#ff0000".to_string();
        } else {
            self.color = "#000000".to_string();
        }
        */
    }

    fn draw(&mut self, context: &CanvasRenderingContext2d, scale: f64){
        context.set_stroke_style(&JsValue::from_str(&self.color));
        let adjusted_width = self.line_width / scale;
        context.set_line_width(adjusted_width);
        context.begin_path();
        context.move_to(self.start.x, self.start.y);
        context.line_to(self.end.x, self.end.y);
        context.stroke();
    }   

    fn draw_xor(&self, context: &CanvasRenderingContext2d){
        context.save();
        context.set_global_composite_operation("xor").expect("something goes wrong when apply xor");

        context.begin_path();
        context.move_to(self.start.x, self.start.y);
        context.line_to(self.end.x, self.end.y);
        context.close_path();

        context.set_stroke_style(&JsValue::from_str(&self.color));
        context.set_line_width(self.line_width);

        context.stroke();
        context.restore();
    }
}

pub struct Svg{
    location: Point2D,
    content: String,

    styles: Option<HashMap<String, HashMap<String, String>>>,
}

impl Svg{
    pub fn new(location: Point2D, svg_text: &str) -> Self {
        Svg{location, content: svg_text.to_string(), styles: None}
    }

    // 🎯 SVG에서 Gradient를 추출하는 함수
    fn extract_gradients(&self, context: &CanvasRenderingContext2d, svg_element: &Element) -> HashMap<String, CanvasGradient> {
        let mut gradients: HashMap<String, CanvasGradient> = std::collections::HashMap::new();

        let linear_gradients = svg_element.query_selector_all("linearGradient").unwrap();
        for i in 0..linear_gradients.length() {
            if let Some(gradient_element) = linear_gradients.item(i) {
                if let Ok(gradient_element) = gradient_element.dyn_into::<Element>() {
                    if let Some(id) = gradient_element.get_attribute("id") {
                        let x1 = gradient_element.get_attribute("x1").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let y1 = gradient_element.get_attribute("y1").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let x2 = gradient_element.get_attribute("x2").unwrap_or("100".to_string()).parse::<f64>().unwrap_or(100.0);
                        let y2 = gradient_element.get_attribute("y2").unwrap_or("100".to_string()).parse::<f64>().unwrap_or(100.0);

                        let gradient = context.create_linear_gradient(x1, y1, x2, y2);

                        let stops = gradient_element.query_selector_all("stop").unwrap();
                        for j in 0..stops.length() {
                            if let Some(stop_element) = stops.item(j) {
                                if let Ok(stop_element) = stop_element.dyn_into::<Element>() {
                                    // 'stop-color' 속성을 가져옵니다.
                                    let stop_color = stop_element
                                        .get_attribute("stop-color")
                                        .unwrap_or_else(|| "black".to_string()); // 기본값은 'black'

                                    // 'stop-opacity' 속성을 가져옵니다.
                                    let stop_opacity = stop_element
                                        .get_attribute("stop-opacity")
                                        .unwrap_or_else(|| "1".to_string()); // 기본값은 '1'

                                    if let Some(offset) = stop_element.get_attribute("offset") {
                                        let offset = offset.trim_end_matches('%').parse::<f32>().unwrap_or(0.0);
                                        gradient.add_color_stop(offset, &stop_color).unwrap();
                                    }
                                    else{
                                        let offset = 0.0;
                                        gradient.add_color_stop(offset, &stop_color).unwrap();
                                    }
                                }
                            }
                        }

                        gradients.insert(id, gradient);
                    }
                }
            }
        }

        let radial_gradients = svg_element.query_selector_all("radialGradient").unwrap();
        for i in 0..radial_gradients.length() {
            if let Some(gradient_element) = radial_gradients.item(i) {
                if let Ok(gradient_element) = gradient_element.dyn_into::<Element>() {
                    if let Some(id) = gradient_element.get_attribute("id") {
                        let cx = gradient_element.get_attribute("x1").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let cy = gradient_element.get_attribute("y1").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
                        let r = gradient_element.get_attribute("r").unwrap_or("1".to_string()).parse::<f64>().unwrap_or(1.0);

                        let transform = gradient_element.get_attribute("gradientTransform").unwrap_or("".to_string());
                        context.save();
                        self.apply_transform(context, &transform);
                        let gradient = context.create_radial_gradient(0.0, 0.0, 0.0, cx, cy, r).unwrap();
                        context.restore();

                        let stops = gradient_element.query_selector_all("stop").unwrap();
                        for j in 0..stops.length() {
                            if let Some(stop_element) = stops.item(j) {
                                if let Ok(stop_element) = stop_element.dyn_into::<Element>() {
                                    // 'stop-color' 속성을 가져옵니다.
                                    let stop_color = stop_element
                                        .get_attribute("stop-color")
                                        .unwrap_or_else(|| "black".to_string()); // 기본값은 'black'

                                    // 'stop-opacity' 속성을 가져옵니다.
                                    let stop_opacity = stop_element
                                        .get_attribute("stop-opacity")
                                        .unwrap_or_else(|| "1".to_string()); // 기본값은 '1'

                                    if let Some(offset) = stop_element.get_attribute("offset") {
                                        let offset = offset.trim_end_matches('%').parse::<f32>().unwrap_or(0.0);
                                        gradient.add_color_stop(offset, &stop_color).unwrap();
                                    }
                                    else{
                                        let offset = 0.0;
                                        gradient.add_color_stop(offset, &stop_color).unwrap();
                                    }
                                }
                            }
                        }

                        gradients.insert(id, gradient);
                    }
                }
            }
        }

        gradients
    }

    // 🎯 SVG 내부에서 `<style>` 태그를 분석하여 스타일 규칙을 저장
    fn extract_styles(&mut self, svg_element: &Element) {
        self.styles = Some(HashMap::new());
        let style_elements = svg_element.query_selector_all("style").unwrap();

        for i in 0..style_elements.length() {
            if let Some(style_element) = style_elements.item(i) {
                if let Some(style_text) = style_element.text_content() {
                    for rule in style_text.split('}') {
                        let parts: Vec<&str> = rule.split('{').collect();
                        if parts.len() == 2 {
                            let selector = parts[0].trim().replace(".", "").to_string();
                            let properties = parts[1].trim();

                            let mut prop_map = std::collections::HashMap::new();
                            for prop in properties.split(';') {
                                let key_value: Vec<&str> = prop.split(':').map(|s| s.trim()).collect();
                                if key_value.len() == 2 {
                                    prop_map.insert(key_value[0].to_string(), key_value[1].to_string());
                                }
                            }

                            self.styles.as_mut().unwrap().insert(selector, prop_map);
                        }
                    }
                }
            }
        }
    }

    // 🎯 CSS 변수를 `window.getComputedStyle()`을 사용하여 해석하는 함수
    fn resolve_css_variable(&self, var_str: &str) -> Option<String> {
        if var_str.starts_with("var(") && var_str.contains("--") {
            let var_content = var_str.trim_start_matches("var(").trim_end_matches(")");
            let parts: Vec<&str> = var_content.split(',').map(|s| s.trim()).collect();

            let css_variable = parts.get(0)?.trim(); // CSS 변수 이름 예: "--vscode-activityBarBadge-background"
            let fallback_color = parts.get(1).map(|s| s.to_string()); // 기본 색상 (옵션)

            let window = window().unwrap();
            let document = window.document().unwrap();
            let body = document.body().unwrap();
            let computed_style = window.get_computed_style(&body).unwrap().unwrap();

            if let Ok(css_value) = computed_style.get_property_value(css_variable) {
                if !css_value.is_empty() {
                    return Some(css_value);
                }
            }

            fallback_color
        } else {
            None
        }
    }

    fn parse_fill_gradient(&self, fill: &String, gradients: &HashMap<String, CanvasGradient>) -> Option<CanvasGradient>{
        if fill.starts_with("url(") {
            let gradient_id = fill.strip_prefix("url(#").and_then(|s| s.strip_suffix(")")).unwrap_or("");
            info!("gradient id: {:?}", gradient_id);

            let cloned = gradients.get(gradient_id).cloned();
            return cloned;
        }

        None
    }

    fn parse_fill_attribute(&self, svg_element: &Element, gradients: &HashMap<String, CanvasGradient>) -> JsValue {
        let fill = svg_element.get_attribute("fill").unwrap_or("".to_string());
        if fill.starts_with("url(") {
            if let Some(gradient) = self.parse_fill_gradient(&fill, gradients){
                return JsValue::from(gradient);
            }
        }else if fill.starts_with("var(") {
            if let Some(resolved_color) = self.resolve_css_variable(&fill) {
                return JsValue::from_str(&resolved_color);
            }
        } else if fill.to_lowercase() != "none" {
            if fill.is_empty() {
                return JsValue::from_str("black");
            }
            else{
                return JsValue::from_str(&fill);
            }
        }

        JsValue::from_str("none")
    }

    // 🎯 SVG를 Canvas에 순서대로 그리는 함수 (g 요소 포함)
    pub fn render_svg_to_canvas(&self, context: &CanvasRenderingContext2d, parent_element: &Element , gradients: &HashMap<String, CanvasGradient>){
        context.save();
        context.translate(self.location.x, self.location.y).unwrap();

        let child_nodes = parent_element.child_nodes();
        for i in 0..child_nodes.length() {
            if let Some(node) = child_nodes.item(i) {
                if let Some(element) = node.dyn_ref::<Element>() {
                    let tag_name = element.tag_name().to_lowercase();
                    let fill_style = self.parse_fill_attribute(element, gradients);

                    match tag_name.as_str() {
                        "g" => self.render_group(&context, element, gradients, &fill_style),
                        "rect" => self.render_rect(&context, element, gradients, &fill_style),
                        "polygon" => self.render_polygon(&context, element, gradients),
                        "polyline" => self.render_polyline(&context, element, gradients),
                        "ellipse" => self.render_ellipse(&context, element, gradients),
                        "circle" => self.render_circle(&context, element, gradients),
                        "path" => self.render_path(&context, element, gradients, &fill_style),
                        "text" => self.render_text(&context, element, gradients),
                        _ => (),
                    }
                }
            }
        }

        context.restore();
    }

    // 🎯 `g` 요소의 `transform` 속성을 적용하는 함수
    fn apply_transform(&self, context: &CanvasRenderingContext2d, transform_str: &str) {
        match transform_str.parse::<Transform>() {
            Ok(transform) => {
                // Iterate over the parsed transform operations
                context.transform(transform.a, transform.b, transform.c, transform.d, transform.e, transform.f).unwrap();
            }
            Err(e) => {
                info!("Failed to parse transform: {:?}", e);
            }
        }

        /*
        if transform_str.starts_with("translate(") {
            let values: Vec<f64> = transform
                .trim_start_matches("translate(")
                .trim_end_matches(")")
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();

            if values.len() == 2 {
                context.translate(values[0], values[1]).unwrap();
            }
        }
        */
    }

    // 🎯 `class` 속성이 있으면 스타일 적용
    fn apply_class_attribute(&self, context: &CanvasRenderingContext2d, svg_element: &Element) {
        if let Some(class_name) = svg_element.get_attribute("class") {
            info!("class_name: {:?}", class_name);

            for class in class_name.split_whitespace() {
                if let Some(class_styles) = self.styles.as_ref().unwrap().get(class) {
                    if let Some(fill) = class_styles.get("fill") {
                        context.set_fill_style(&JsValue::from_str(fill));

                        info!("apply_class_attribute fill: {:?}", fill);
                    }
                    if let Some(stroke) = class_styles.get("stroke") {
                        let stroke_style = JsValue::from_str(stroke);
                        context.set_stroke_style(&stroke_style);

                        info!("stroke: {:?}", stroke);
                    }
                    if let Some(opacity) = class_styles.get("opacity") {
                        // Set the global alpha to the specified opacity
                        let opacity_value = opacity.parse::<f64>().unwrap_or(1.0);
                        context.set_global_alpha(opacity_value);
                        info!("opacity: {:?}", opacity_value);
                    }
                }
            }
        }
    }

    // 🎯 `fill` 속성이 있으면 스타일 적용
    fn apply_fill_attribute(&self, context: &CanvasRenderingContext2d, svg_element: &Element, gradients: &HashMap<String, CanvasGradient>) -> bool{
        let fill_style = self.parse_fill_attribute(svg_element, gradients);
        if fill_style.as_string().unwrap_or_default() != "none" {
            info!("apply_fill_attribute {:?}", fill_style.as_string().unwrap_or_default());
            context.set_fill_style(&fill_style);
            return true;
        }

        false
    }

    // 🎯 Group 요소 처리
    fn render_group(&self, context: &CanvasRenderingContext2d, group_element: &Element, gradients: &HashMap<String, CanvasGradient>, fill_style: &JsValue){
        context.save();

        let transform = group_element.get_attribute("transform").unwrap_or_default();
        self.apply_transform(context, &transform);
        self.apply_class_attribute(&context, group_element);
        self.apply_fill_attribute(&context, group_element, gradients);

        // 🎨 그룹의 `fill` 속성 가져오기
        let mut group_fill = self.parse_fill_attribute(group_element, gradients);
        if group_fill.as_string().unwrap_or_default() == "none" {
            group_fill = fill_style.clone();
        }

        let child_nodes = group_element.child_nodes();
        for i in 0..child_nodes.length() {
            if let Some(node) = child_nodes.item(i) {
                if let Some(element) = node.dyn_ref::<Element>() {
                    let tag_name = element.tag_name().to_lowercase();
                    match tag_name.as_str() {
                        "g" => self.render_group(&context, element, gradients, &group_fill),
                        "rect" => self.render_rect(&context, element, gradients, &group_fill),
                        "polygon" => self.render_polygon(&context, element, gradients),
                        "polyline" => self.render_polyline(&context, element, gradients),
                        "ellipse" => self.render_ellipse(&context, element, gradients),
                        "circle" => self.render_circle(&context, element, gradients),
                        "path" => self.render_path(&context, element, gradients, &group_fill),
                        "text" => self.render_text(&context, element, gradients),
                        _ => (),
                    }
                }
            }
        }

        context.restore();
    }

    // 🎯 Polygon 요소 처리
    fn render_polygon(&self, context: &CanvasRenderingContext2d, polygon_element: &Element, gradients: &HashMap<String, CanvasGradient>){
        if let Some(points) = polygon_element.get_attribute("points") {
            let points_vec: Vec<&str> = points.split_whitespace().collect();
            if points_vec.len() >= 2 {
                context.save();
                context.begin_path();

                // 🎯 첫 번째 점으로 이동
                if let Some(first_point) = points_vec.get(0) {
                    let coords: Vec<f64> = first_point.split(',')
                        .filter_map(|s| s.parse::<f64>().ok())
                        .collect();
                    if coords.len() == 2 {
                        context.move_to(coords[0], coords[1]);
                    }
                }

                // 🎯 나머지 점들을 선으로 연결
                for point in points_vec.iter().skip(1) {
                    let coords: Vec<f64> = point.split(',')
                        .filter_map(|s| s.parse::<f64>().ok())
                        .collect();
                    if coords.len() == 2 {
                        context.line_to(coords[0], coords[1]);
                    }
                }

                context.close_path();

                // 🎨 색상 처리
                self.apply_class_attribute(context, polygon_element);
                let filled = self.apply_fill_attribute(context, polygon_element, gradients);
                if filled{ context.fill(); }

                let stroke_color = polygon_element.get_attribute("stroke").unwrap_or("none".to_string());
                if !stroke_color.is_empty() && stroke_color.to_lowercase() != "none" {
                    context.set_stroke_style(&JsValue::from_str(&stroke_color));
                    context.stroke();
                }

                context.restore();
            }
        }
    }
    
    // 🎯 `polyline` 요소를 Canvas에 그리는 함수
    fn render_polyline(&self, context: &CanvasRenderingContext2d, polyline_element: &Element, gradients: &HashMap<String, CanvasGradient>) {
        if let Some(points) = polyline_element.get_attribute("points") {
            let points_vec: Vec<&str> = points.split_whitespace().collect();
            if points_vec.len() >= 2 {
                context.save();
                context.begin_path();

                if let Some(first_point) = points_vec.get(0) {
                    let coords: Vec<f64> = first_point.split(',')
                        .filter_map(|s| s.parse::<f64>().ok())
                        .collect();
                    if coords.len() == 2 {
                        context.move_to(coords[0], coords[1]);
                    }
                }

                for point in points_vec.iter().skip(1) {
                    let coords: Vec<f64> = point.split(',')
                        .filter_map(|s| s.parse::<f64>().ok())
                        .collect();
                    if coords.len() == 2 {
                        context.line_to(coords[0], coords[1]);
                    }
                }
                context.close_path();

                self.apply_class_attribute(context, polyline_element);
                let filled = self.apply_fill_attribute(context, polyline_element, gradients);
                if filled{
                    context.fill();
                }

                let stroke_color = polyline_element.get_attribute("stroke").unwrap_or("none".to_string());
                if !stroke_color.is_empty() && stroke_color.to_lowercase() != "none" {
                    context.set_stroke_style(&JsValue::from_str(&stroke_color));
                    context.stroke();
                }

                context.restore();
            }
        }
    }

    // 🎯 Ellipse 요소 처리
    fn render_ellipse(&self, context: &CanvasRenderingContext2d, ellipse_element: &Element, gradients: &HashMap<String, CanvasGradient>){
        let cx = ellipse_element.get_attribute("cx").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let cy = ellipse_element.get_attribute("cy").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let rx = ellipse_element.get_attribute("rx").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let ry = ellipse_element.get_attribute("ry").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let stroke_color = ellipse_element.get_attribute("stroke").unwrap_or("none".to_string());

        context.save();
        context.begin_path();
        context.ellipse(cx, cy, rx, ry, 0.0, 0.0, std::f64::consts::PI * 2.0).unwrap();
        context.close_path();

        self.apply_class_attribute(&context, ellipse_element);
        self.apply_fill_attribute(&context, ellipse_element, gradients);

        context.fill();

        if !stroke_color.is_empty() && stroke_color.to_lowercase() != "none" {
            context.set_stroke_style(&JsValue::from_str(&stroke_color));
            context.stroke();
        }

        context.restore();
    }

    // 🎯 `circle` 요소를 Canvas에 그리는 함수
    fn render_circle(&self, context: &CanvasRenderingContext2d, circle_element: &Element , gradients: &HashMap<String, CanvasGradient>) {
        let cx = circle_element.get_attribute("cx").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let cy = circle_element.get_attribute("cy").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let r = circle_element.get_attribute("r").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let mut fill_style = JsValue::from_str("none");
        let mut stroke_style= JsValue::from_str(circle_element.get_attribute("stroke").unwrap_or("none".to_string()).as_str());
        let mut opacity_style = JsValue::from_str("1.0");

        let fill_rule = circle_element.get_attribute("fill-rule").unwrap_or("nonzero".to_string());

        context.save();

        self.apply_class_attribute(context, circle_element);
        self.apply_fill_attribute(context, circle_element, gradients);

        context.begin_path();
        context.arc(cx, cy, r, 0.0, std::f64::consts::PI * 2.0).unwrap();
        context.close_path();

        // 🎯 Fill 적용
        context.fill();

        // 🎯 Stroke 적용
        if stroke_style.as_string().unwrap_or_default().to_lowercase() != "none" {
            context.set_stroke_style(&stroke_style);
            context.stroke();
        }

        context.restore();
    }

    fn render_rect(&self, context: &CanvasRenderingContext2d, rect_element: &Element, gradients: &HashMap<String, CanvasGradient>, group_fill: &JsValue){
        let x_pos = rect_element.get_attribute("x").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let y_pos = rect_element.get_attribute("y").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let width = rect_element.get_attribute("width").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let height = rect_element.get_attribute("height").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let rx = rect_element.get_attribute("rx").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let ry = rect_element.get_attribute("ry").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);

        let mut fill_style= group_fill.clone();
        fill_style = self.parse_fill_attribute(rect_element, gradients);
        if fill_style.as_string().unwrap_or_default() == "none" {
            fill_style = group_fill.clone();
        }
        let stroke_color = rect_element.get_attribute("stroke").unwrap_or("none".to_string());

        context.save();

        let transform = rect_element.get_attribute("transform").unwrap_or_default();
        self.apply_transform(context, &transform);

        context.begin_path();

        if rx > 0.0 && rx == width * 0.5 && rx == height * 0.5 {
            // 🎯 원 (Circle)
            let cx = x_pos + width / 2.0;
            let cy = y_pos + height / 2.0;
            context.arc(cx,cy, rx, 0.0, std::f64::consts::PI * 2.0).unwrap();
        } else if rx > 0.0 || ry > 0.0 {
            // 🎯 모서리가 둥근 사각형 (Rounded Rectangle)
            context.move_to(x_pos + rx, y_pos);
            context.line_to(x_pos + width - rx, y_pos);
            context.quadratic_curve_to(x_pos + width, y_pos, x_pos + width, y_pos + ry);
            context.line_to(x_pos + width, y_pos + height - ry);
            context.quadratic_curve_to(x_pos + width, y_pos + height, x_pos + width - rx, y_pos + height);
            context.line_to(x_pos + rx, y_pos + height);
            context.quadratic_curve_to(x_pos, y_pos + height, x_pos, y_pos + height - ry);
            context.line_to(x_pos, y_pos + ry);
            context.quadratic_curve_to(x_pos, y_pos, x_pos + rx, y_pos);
            context.close_path();
        } else {
            // 🎯 일반 사각형
            context.rect(x_pos, y_pos, width, height);
        }

        // 🎨 색상 처리
        if fill_style.as_string().unwrap_or_default().to_lowercase() != "none" {
            context.set_fill_style(&fill_style);
            context.fill();
        }

        if !stroke_color.is_empty() && stroke_color.to_lowercase() != "none" {
            context.set_stroke_style(&JsValue::from_str(&stroke_color));
            context.stroke();
        }

        context.restore();
    }

    // 🎯 `path` 요소를 Canvas에 그리는 함수
    fn render_path(&self, context: &CanvasRenderingContext2d, path_element: &Element, gradients: &HashMap<String, CanvasGradient>, group_fill: &JsValue) {
        if let Some(d_attr) = path_element.get_attribute("d") {
            if let Ok(path) = Path2d::new_with_path_string(&d_attr) {
                // 🎨 SVG 색상 적용 (fill, stroke)
                let mut fill_style= group_fill.clone();

                let fill_rule = path_element.get_attribute("fill-rule").unwrap_or("nonzero".to_string());

                // 🎯 드롭된 위치에 그리기
                context.save();

                // Set the global alpha to the specified opacity
                if let Some(opacity_attr) = path_element.get_attribute("opacity") {
                    let opacity_value = opacity_attr.parse::<f64>().unwrap_or(1.0);
                    context.set_global_alpha(opacity_value);
                }

                self.apply_class_attribute(&context, path_element);
                let filled = self.apply_fill_attribute(&context, path_element, gradients);
                if filled{
                    context.fill_with_path_2d(&path);
                }

                let clip_rule = path_element.get_attribute("clip-rule").unwrap_or("nonzero".to_string());
                if clip_rule == "evenodd" {
                    info!("clip_rule: {:?}", clip_rule);
                    context.clip_with_path_2d(&path);
                }

                let stroke_style= JsValue::from_str(path_element.get_attribute("stroke").unwrap_or("none".to_string()).as_str());
                if stroke_style.as_string().unwrap_or_default().to_lowercase() != "none" {
                    context.set_stroke_style(&stroke_style);
                    context.stroke_with_path(&path);
                }

                // fill, style 속성이 없을 경우 처리
                /*
                if fill_style.as_string().unwrap_or_default().to_lowercase() == "none" 
                && stroke_style.as_string().unwrap_or_default().to_lowercase() == "none" 
                && path_element.get_attribute("fill").is_none()
                && path_element.get_attribute("stroke").is_none() {
                    fill_style = JsValue::from_str("black");
                    context.set_fill_style(&fill_style);
                    context.fill_with_path_2d(&path);
                }
                */

                context.restore();
            } else {
                web_sys::console::log_1(&JsValue::from_str(&format!("⚠️ Path2d 변환 실패: {}", d_attr)));
            } 
        }
    }

    // 🎯 `text` 요소를 Canvas에 그리는 함수
    fn render_text(&self, context: &CanvasRenderingContext2d, text_element: &Element, gradients: &HashMap<String, CanvasGradient>) {
        let text_content = text_element.text_content().unwrap_or_default();
        let x_pos = text_element.get_attribute("x").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let y_pos = text_element.get_attribute("y").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let font_size = text_element.get_attribute("font-size").unwrap_or("16".to_string());
        let font_family = text_element.get_attribute("font-family").unwrap_or("Arial".to_string());
        let text_anchor = text_element.get_attribute("text-anchor").unwrap_or("start".to_string());
        let fill_color = text_element.get_attribute("fill").unwrap_or("none".to_string());
        let stroke_color = text_element.get_attribute("stroke").unwrap_or("black".to_string());

        context.save();

        // 🎯 Font 설정
        let font_style = format!("{}px {}", font_size, font_family);
        context.set_font(&font_style);

        // 🎯 Text 정렬 처리
        let text_align = match text_anchor.as_str() {
            "middle" => "center",
            "end" => "right",
            _ => "left",
        };
        context.set_text_align(text_align);

        // Iterate over <tspan> elements
        let tspans = text_element.get_elements_by_tag_name("tspan");
        if tspans.length() > 0{
            for i in 0..tspans.length() {
                if let Some(tspan) = tspans.item(i) {
                    let tspan_element = tspan.dyn_into::<Element>().unwrap();

                    // Retrieve <tspan> specific attributes
                    let tspan_x = tspan_element
                        .get_attribute("x")
                        .and_then(|v| v.parse::<f64>().ok())
                        .unwrap_or(x_pos);
                    let tspan_y = tspan_element
                        .get_attribute("y")
                        .and_then(|v| v.parse::<f64>().ok())
                        .unwrap_or(y_pos);

                    // Retrieve the text content of the <tspan>
                    if let Some(text_content) = tspan_element.text_content() {
                        // Render the text at the specified position
                        // 🎨 Stroke 적용
                        if stroke_color.to_lowercase() != "none" {
                            context.set_stroke_style(&JsValue::from_str(&stroke_color));
                            context
                                .fill_text(&text_content, tspan_x, tspan_y)
                                .unwrap();
                        }

                        // 🎨 Fill 적용
                        if fill_color.to_lowercase() != "none" {
                            context.set_fill_style(&JsValue::from_str(&fill_color));
                            context.fill_text(&text_content, tspan_x, tspan_y).unwrap();
                        }
                    }
                }
            }
        }
        else{
            // 🎨 Stroke 적용
            if stroke_color.to_lowercase() != "none" {
                context.set_stroke_style(&JsValue::from_str(&stroke_color));
                context.stroke_text(&text_content, x_pos, y_pos).unwrap();
            }

            // 🎨 Fill 적용
            let filled = self.apply_fill_attribute(context, text_element, gradients);
            if filled {
                context.fill_text(&text_content, x_pos, y_pos).unwrap();
            }
        }

        context.restore();
    }
}

impl Shape for Svg{
    fn color(&self) -> &str {
        "#0000ff"
    }

    fn line_width(&self) -> f64 {
        2.0
    }

    fn is_hit(&self, x: f64, y: f64) -> bool {
        false        
    }

    fn set_hovered(&mut self, hovered: bool) {
        if hovered {
            self.content = r#"
                <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
                    <rect x="10" y="10" width="80" height="80" fill="red" />
                </svg>
            "#.to_string();
        } else {
            self.content = r#"
                <svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
                    <rect x="10" y="10" width="80" height="80" fill="blue" />
                </svg>
            "#.to_string();
        }
    }

    fn draw(&mut self, context: &CanvasRenderingContext2d, scale: f64){
        let parser = DomParser::new().unwrap();
        let doc = parser.parse_from_string(&self.content, web_sys::SupportedType::ImageSvgXml).unwrap();

        if let Some(svg_element) = doc.query_selector("svg").ok().flatten() {
            let gradients = self.extract_gradients(context, &svg_element);
            self.extract_styles(&svg_element);
            self.render_svg_to_canvas(context, &svg_element, &gradients);
        } else {
            web_sys::console::log_1(&"⚠️ SVG 파싱 실패".into());
        }
    }

    fn draw_xor(&self, context: &CanvasRenderingContext2d){
    }
}