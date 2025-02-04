use std::collections::HashMap;
use std::str;
use std::task::Context;
use log::info;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_sys::console::group;
use web_sys::{window, CanvasRenderingContext2d, Element, DomParser, CanvasGradient, HtmlCanvasElement, Path2d};

#[derive(Clone, Copy)]
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
    fn draw(&mut self, context: &CanvasRenderingContext2d);
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

    fn draw(&mut self, context: &CanvasRenderingContext2d){
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

    fn draw(&mut self, context: &CanvasRenderingContext2d){
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

    styles: Option<HashMap<String, HashMap<String, String>>>,
}

impl Svg{
    pub fn new(location: Point2D, svg_text: &str) -> Self {
        Svg{location, content: svg_text.to_string(), styles: None}
    }

    // 🎯 SVG에서 Gradient를 추출하는 함수
    fn extract_gradients(svg_element: &Element) -> HashMap<String, CanvasGradient> {
        let mut gradients = std::collections::HashMap::new();
        let context = window().unwrap().document().unwrap().get_element_by_id("drawing-canvas")
            .unwrap()
            .dyn_into::<HtmlCanvasElement>().unwrap()
            .get_context("2d").unwrap()
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>().unwrap();

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
                        info!("stops.length(): {}", stops.length());

                        for j in 0..stops.length() {
                            if let Some(stop_element) = stops.item(j) {
                                if let Ok(stop_element) = stop_element.dyn_into::<Element>() {
                                    if let Some(offset) = stop_element.get_attribute("offset") {
                                        let offset = offset.trim_end_matches('%').parse::<f32>().unwrap_or(0.0) / 100.0;
                                        info!("offset: {}", offset);

                                        if let Some(color) = stop_element.get_attribute("stop-color") {
                                            info!("color: {}", color);
                                            gradient.add_color_stop(offset, &color).unwrap();
                                        }
                                    }
                                    else{
                                        let offset = 0.0;
                                        if let Some(color) = stop_element.get_attribute("stop-color") {
                                            info!("color: {}", color);
                                            gradient.add_color_stop(offset, &color).unwrap();
                                        }
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

    fn parse_fill_attribute(&self, svg_element: &Element, gradients: &HashMap<String, CanvasGradient>) -> JsValue {
        let fill = svg_element.get_attribute("fill").unwrap_or("none".to_string());
        if fill.starts_with("url(") {
            let gradient_id = fill.strip_prefix("url(#").and_then(|s| s.strip_suffix(")")).unwrap_or("");
            if let Some(gradient) = gradients.get(gradient_id) {
                return JsValue::from(gradient);
            }
        } else if fill.to_lowercase() != "none" {
            return JsValue::from_str(&fill);
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
                        "g" => self.render_group(context, element, gradients, &fill_style),
                        "rect" => self.render_rect(context, element),
                        "polygon" => self.render_polygon(context, element),
                        "polyline" => self.render_polyline(context, element),
                        "ellipse" => self.render_ellipse(context, element),
                        "circle" => self.render_circle(context, element, gradients),
                        "path" => self.render_path(context, element, gradients, &fill_style),
                        "text" => self.render_text(context, element),
                        _ => (),
                    }
                }
            }
        }

        context.restore();
    }

    // 🎯 `g` 요소의 `transform` 속성을 적용하는 함수
    fn apply_transform(&self, context: &CanvasRenderingContext2d, transform: &str) {
        if transform.starts_with("translate(") {
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
    }

    // 🎯 Group 요소 처리
    fn render_group(&self, context: &CanvasRenderingContext2d, group_element: &Element, gradients: &HashMap<String, CanvasGradient>, fill_style: &JsValue){
        let transform = group_element.get_attribute("transform").unwrap_or_default();
        self.apply_transform(context, &transform);

        context.save();
        self.apply_transform(context, &transform);

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
                    let transform = element.get_attribute("transform").unwrap_or_default();

                    context.save();
                    self.apply_transform(context, &transform);
                    
                    match tag_name.as_str() {
                        "g" => self.render_group(context, element, gradients, &group_fill),
                        "rect" => self.render_rect(context, element),
                        "polygon" => self.render_polygon(context, element),
                        "polyline" => self.render_polyline(context, element),
                        "ellipse" => self.render_ellipse(context, element),
                        "circle" => self.render_circle(context, element, gradients),
                        "path" => self.render_path(context, element, gradients, &group_fill),
                        "text" => self.render_text(context, element),
                        _ => (),
                    }

                    context.restore();
                }
            }
        }

        context.restore();
    }

    // 🎯 Polygon 요소 처리
    fn render_polygon(&self, context: &CanvasRenderingContext2d, polygon_element: &Element){
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
                let fill_color = polygon_element.get_attribute("fill").unwrap_or("none".to_string());
                let stroke_color = polygon_element.get_attribute("stroke").unwrap_or("none".to_string());

                if fill_color.to_lowercase() != "none" {
                    context.set_fill_style(&JsValue::from_str(&fill_color));
                    context.fill();
                }

                if !stroke_color.is_empty() && stroke_color.to_lowercase() != "none" {
                    context.set_stroke_style(&JsValue::from_str(&stroke_color));
                    context.stroke();
                }

                context.restore();
            }
        }
    }
    
    // 🎯 `polyline` 요소를 Canvas에 그리는 함수
    fn render_polyline(&self, context: &CanvasRenderingContext2d, polyline_element: &Element) {
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

                let fill_color = polyline_element.get_attribute("fill").unwrap_or("none".to_string());
                let stroke_color = polyline_element.get_attribute("stroke").unwrap_or("none".to_string());

                if fill_color.to_lowercase() != "none" {
                    context.set_fill_style(&JsValue::from_str(&fill_color));
                    context.fill();
                }
                if !stroke_color.is_empty() && stroke_color.to_lowercase() != "none" {
                    context.set_stroke_style(&JsValue::from_str(&stroke_color));
                    context.stroke();
                }

                context.restore();
            }
        }
    }

    // 🎯 Ellipse 요소 처리
    fn render_ellipse(&self, context: &CanvasRenderingContext2d, ellipse_element: &Element){
        let cx = ellipse_element.get_attribute("cx").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let cy = ellipse_element.get_attribute("cy").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let rx = ellipse_element.get_attribute("rx").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let ry = ellipse_element.get_attribute("ry").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let fill_color = ellipse_element.get_attribute("fill").unwrap_or("none".to_string());
        info!("fill_color: {}", fill_color);
        let stroke_color = ellipse_element.get_attribute("stroke").unwrap_or("none".to_string());

        context.save();
        context.begin_path();
        context.ellipse(cx, cy, rx, ry, 0.0, 0.0, std::f64::consts::PI * 2.0).unwrap();

        if fill_color.to_lowercase() != "none" {
            context.set_fill_style(&JsValue::from_str(&fill_color));
            context.fill();
        }

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

        if let Some(fill) = circle_element.get_attribute("fill") {
            if fill.starts_with("url(") {
                if let Some(gradient_id) = fill.strip_prefix("url(#").and_then(|s| s.strip_suffix(")")) {
                    if let Some(gradient) = gradients.get(gradient_id) {
                        fill_style = JsValue::from(gradient);
                        info!("gradient_id: {:?}", gradient_id);
                    }
                }
            } else if fill.to_lowercase() != "none" {
                fill_style = JsValue::from_str(&fill);
            }
        }

        let fill_rule = circle_element.get_attribute("fill-rule").unwrap_or("nonzero".to_string());

        // 🎯 `class` 속성이 있으면 스타일 적용
        if let Some(class_name) = circle_element.get_attribute("class") {
            for class in class_name.split_whitespace() {
                if let Some(class_styles) = self.styles.as_ref().unwrap().get(class) {
                    if let Some(fill) = class_styles.get("fill") {
                        fill_style = JsValue::from_str(fill);
                    }
                    if let Some(stroke) = class_styles.get("stroke") {
                        stroke_style = JsValue::from_str(stroke);
                    }
                }
                else{
                    fill_style = JsValue::from_str("black");
                }
            }
        }

        context.save();
        context.begin_path();
        context.arc(cx, cy, r, 0.0, std::f64::consts::PI * 2.0).unwrap();

        // 🎯 Fill 적용
        if fill_style.as_string().unwrap_or_default() != "none" {
            context.set_fill_style(&fill_style);
            context.fill();
        }

        // 🎯 Stroke 적용
        if stroke_style.as_string().unwrap_or_default().to_lowercase() != "none" {
            context.set_stroke_style(&stroke_style);
            context.stroke();
        }

        context.restore();
    }

    fn render_rect(&self, context: &CanvasRenderingContext2d, rect_element: &Element){
        let x_pos = rect_element.get_attribute("x").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let y_pos = rect_element.get_attribute("y").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let width = rect_element.get_attribute("width").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let height = rect_element.get_attribute("height").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let rx = rect_element.get_attribute("rx").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let ry = rect_element.get_attribute("ry").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);

        let fill_color = rect_element.get_attribute("fill").unwrap_or("none".to_string());
        let stroke_color = rect_element.get_attribute("stroke").unwrap_or("none".to_string());

        context.save();
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
        if fill_color.to_lowercase() != "none" {
            context.set_fill_style(&JsValue::from_str(&fill_color));
            context.fill();
        }

        if !stroke_color.is_empty() && stroke_color.to_lowercase() != "none" {
            context.set_stroke_style(&JsValue::from_str(&stroke_color));
            context.stroke();
        }

        context.restore();
    }

    fn render_path(&self, context: &CanvasRenderingContext2d, path_element: &Element, gradients: &HashMap<String, CanvasGradient>, group_fill: &JsValue) {
        if let Some(d_attr) = path_element.get_attribute("d") {
            if let Ok(path) = Path2d::new_with_path_string(&d_attr) {
                // 🎨 SVG 색상 적용 (fill, stroke)
                let mut fill_style= group_fill.clone();
                let mut stroke_style= JsValue::from_str(path_element.get_attribute("stroke").unwrap_or("none".to_string()).as_str());

                fill_style = self.parse_fill_attribute(path_element, gradients);
                let fill_rule = path_element.get_attribute("fill-rule").unwrap_or("nonzero".to_string());
                let clip_rule = path_element.get_attribute("clip-rule").unwrap_or("nonzero".to_string());

                // 🎯 `class` 속성이 있으면 스타일 적용
                if let Some(class_name) = path_element.get_attribute("class") {
                    for class in class_name.split_whitespace() {
                        if let Some(class_styles) = self.styles.as_ref().unwrap().get(class) {
                            if let Some(fill) = class_styles.get("fill") {
                                fill_style = JsValue::from_str(fill);
                                info!("fill_style: {:?}", fill);
                            }
                            if let Some(stroke) = class_styles.get("stroke") {
                                stroke_style = JsValue::from_str(stroke);
                            }
                        }
                        else{
                            fill_style = JsValue::from_str("black");
                        }
                    }
                }

                // 🎯 드롭된 위치에 그리기
                context.save();

                if fill_style.as_string().unwrap_or_default() != "none" {
                    context.set_fill_style(&fill_style);
                    context.fill_with_path_2d(&path);

                    if clip_rule == "evenodd" {
                        info!("clip_rule: {:?}", clip_rule);
                        context.clip_with_path_2d(&path);
                    }
                }

                if stroke_style.as_string().unwrap_or_default().to_lowercase() != "none" {
                    context.set_stroke_style(&stroke_style);
                    context.stroke_with_path(&path);
                }

                context.restore();
            } else {
                web_sys::console::log_1(&JsValue::from_str(&format!("⚠️ Path2d 변환 실패: {}", d_attr)));
            } 
        }
    }

    // 🎯 `text` 요소를 Canvas에 그리는 함수
    fn render_text(&self, context: &CanvasRenderingContext2d, text_element: &Element) {
        let text_content = text_element.text_content().unwrap_or_default();
        let x_pos = text_element.get_attribute("x").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let y_pos = text_element.get_attribute("y").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let font_size = text_element.get_attribute("font-size").unwrap_or("16".to_string());
        let font_family = text_element.get_attribute("font-family").unwrap_or("Arial".to_string());
        let text_anchor = text_element.get_attribute("text-anchor").unwrap_or("start".to_string());
        let fill_color = text_element.get_attribute("fill").unwrap_or("none".to_string());
        let stroke_color = text_element.get_attribute("stroke").unwrap_or("none".to_string());

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

        context.save();

        // 🎨 Stroke 적용
        if stroke_color.to_lowercase() != "none" {
            context.set_stroke_style(&JsValue::from_str(&stroke_color));
            context.stroke_text(&text_content, x_pos, y_pos).unwrap();
        }

        // 🎨 Fill 적용
        if fill_color.to_lowercase() != "none" {
            context.set_fill_style(&JsValue::from_str(&fill_color));
            context.fill_text(&text_content, x_pos, y_pos).unwrap();
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

    fn draw(&mut self, context: &CanvasRenderingContext2d){
        let parser = DomParser::new().unwrap();
        let doc = parser.parse_from_string(&self.content, web_sys::SupportedType::ImageSvgXml).unwrap();

        if let Some(svg_element) = doc.query_selector("svg").ok().flatten() {
            let gradients = Svg::extract_gradients(&svg_element);
            self.extract_styles(&svg_element);
            self.render_svg_to_canvas(context, &svg_element, &gradients);
        } else {
            web_sys::console::log_1(&"⚠️ SVG 파싱 실패".into());
        }
    }
}