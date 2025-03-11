use std::any::Any;
use std::collections::HashMap;
use std::f64::MAX;
use std::f64::consts::PI;
use std::iter::Scan;
use std::str;
use std::task::Context;
use std::thread::panicking;
use log::info;
use piet::LinearGradient;
use piet_web::WebRenderContext;
use js_sys::Promise;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use std::sync::{Arc, Mutex};

use piet::{RenderContext, Color, Text, TextLayout, StrokeStyle};
use kurbo::Affine;

use web_sys::console::group;
use web_sys::console::info;
use web_sys::{window, CanvasRenderingContext2d, Element, DomParser, CanvasGradient, HtmlCanvasElement, Path2d, CssStyleDeclaration, File, FileReader, Blob};
use svgtypes::Transform;

use crate::state::State;
use super::geometry::{Point2D, Vector2D, BoundingRect2D};
use super::shape::{DrawShape, hex_to_color};
use crate::shapes::{pencil::Pencil, line::Line, rectangle::Rectangle, polyline::Polyline, ellipse::Ellipse, elliptical_arc::EllipticalArc, 
    cubic_bez::CubicBezier, text_box::TextBox, text_box::TextBoxManager};

use crate::vec_draw_doc::VecDrawDoc;

#[derive(Debug, Clone)]
pub struct Svg {
    selected: bool,
    location: Point2D,
    width: f64,
    height: f64,
    content: String,
    selected_control_point: i32,

    styles: Option<HashMap<String, HashMap<String, String>>>,

    shapes: Vec<Arc<Mutex<dyn DrawShape>>>,    // ✅ Shape 리스트
}

impl Svg{
    pub fn new(location: Point2D, svg_text: &str) -> Self {
        Svg{
            selected: false, 
            location, 
            width: f64::MAX,
            height: f64::MAX,
            selected_control_point: -1,
            content: svg_text.to_string(), 
            styles: None,
            shapes: Vec::new()}
    }

    // 🎯 SVG에서 Gradient를 추출하는 함수
    fn extract_gradients(&self, context: &WebRenderContext, svg_element: &Element) -> HashMap<String, CanvasGradient> {
        let mut gradients: HashMap<String, CanvasGradient> = std::collections::HashMap::new();

        /*
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
        */

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
    pub fn render_svg_to_canvas(&mut self, context: &mut WebRenderContext, parent_element: &Element , gradients: &HashMap<String, CanvasGradient>){
        //context.save();
        //context.transform(Affine::new([1.0, 0.0, 0.0, 1.0, self.location.x, self.location.y]));

        let child_nodes = parent_element.child_nodes();
        for i in 0..child_nodes.length() {
            if let Some(node) = child_nodes.item(i) {
                if let Some(element) = node.dyn_ref::<Element>() {
                    let tag_name = element.tag_name().to_lowercase();
                    let fill_style = self.parse_fill_attribute(element, gradients);

                    match tag_name.as_str() {
                        //"g" => self.render_group(&context, element, gradients, &fill_style),
                        "rect" => {
                            if let Some(rectangle) = self.render_rect(&context, element, gradients, &fill_style){
                                self.shapes.push(Arc::new(Mutex::new(rectangle)));
                            }
                        },
                        //"polygon" => self.render_polygon(&context, element, gradients),
                        //"polyline" => self.render_polyline(&context, element, gradients),
                        //"ellipse" => self.render_ellipse(&context, element, gradients),
                        //"circle" => self.render_circle(&context, element, gradients),
                        //"path" => self.render_path(&context, element, gradients, &fill_style),
                        //"text" => self.render_text(&context, element, gradients),
                        _ => (),
                    }
                }
            }
        }

        //context.restore();
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
    fn render_group(&mut self, context: &WebRenderContext, group_element: &Element, gradients: &HashMap<String, CanvasGradient>, fill_style: &JsValue){
        //context.save();

        let transform = group_element.get_attribute("transform").unwrap_or_default();
        //self.apply_transform(context, &transform);
        //self.apply_class_attribute(&context, group_element);
        //self.apply_fill_attribute(&context, group_element, gradients);

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
                        //"g" => self.render_group(&context, element, gradients, &group_fill),
                        "rect" => {
                            if let Some(rectangle) = self.render_rect(&context, element, gradients, &group_fill){
                                self.shapes.push(Arc::new(Mutex::new(rectangle)));
                            }
                        },
                        //"polygon" => self.render_polygon(&context, element, gradients),
                        //"polyline" => self.render_polyline(&context, element, gradients),
                        //"ellipse" => self.render_ellipse(&context, element, gradients),
                        //"circle" => self.render_circle(&context, element, gradients),
                        //"path" => self.render_path(&context, element, gradients, &group_fill),
                        //"text" => self.render_text(&context, element, gradients),
                        _ => (),
                    }
                }
            }
        }

        //context.restore();
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

    fn render_rect(&self, context: &WebRenderContext, rect_element: &Element, gradients: &HashMap<String, CanvasGradient>, group_fill: &JsValue) -> Option<Rectangle>{
        let x_pos = rect_element.get_attribute("x").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let y_pos = rect_element.get_attribute("y").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let width = rect_element.get_attribute("width").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let height = rect_element.get_attribute("height").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let rx = rect_element.get_attribute("rx").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
        let ry = rect_element.get_attribute("ry").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);

        Some(Rectangle::new(Point2D::new(x_pos, y_pos), width, height, "#000000".to_string(), 1.0, Some("".to_string())))

        /*
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
        */
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

impl DrawShape for Svg{
    fn color(&self) -> &str {
        "#0000ff"
    }

    fn line_width(&self) -> f64 {
        2.0
    }

    fn bounding_rect(&self) -> super::geometry::BoundingRect2D {
        BoundingRect2D { min: self.min_point(), max: self.max_point() }
    }

    fn is_hit(&self, x: f64, y: f64, scale: f64) -> bool {
        false        
    }

    /// Given a shape and a point, returns the closest position on the shape's
    /// perimeter, or `None` if the shape is malformed.
    fn closest_perimeter_point(&self, pt: Point2D) -> Option<Point2D> {
        None
    }

    fn max_point(&self) -> Point2D{
        Point2D::new(f64::MAX, f64::MAX)
    }

    fn min_point(&self) -> Point2D{
        Point2D::new(f64::MIN, f64::MIN)
    }

    fn get_control_point(&self, x: f64, y: f64, scale: f64) -> i32{
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

    fn set_hovered(&mut self, hovered: bool) {
        if hovered {
        } else {
        }
    }

    fn move_by(&mut self, dx: f64, dy: f64) {
        
    }

    fn move_control_point_by(&mut self, index: i32, dx: f64, dy: f64) {
    }

    fn draw(&self, context: &mut WebRenderContext, scale: f64){
        self.shapes.iter().for_each(|shape| {
            shape.lock().unwrap().draw(context, scale);
        });

        /*
        let parser = DomParser::new().unwrap();
        let doc = parser.parse_from_string(&self.content, web_sys::SupportedType::ImageSvgXml).unwrap();

        if let Some(svg_element) = doc.query_selector("svg").ok().flatten() {
            let gradients = self.extract_gradients(context, &svg_element);
            self.extract_styles(&svg_element);
            self.render_svg_to_canvas(context, &svg_element, &gradients);
        } else {
            web_sys::console::log_1(&"⚠️ SVG 파싱 실패".into());
        }
        */
    }

    fn draw_xor(&self, context: &mut WebRenderContext, state: &State){
    }

    fn draw_control_points(&self, context: &mut WebRenderContext, scale: f64) {
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

pub async fn parse_svg_file<'a>(context: &mut WebRenderContext<'a>, canvas: &Element, file: File, drop_x: f64, drop_y: f64) -> 
Result<Vec<Box<dyn DrawShape>>, JsValue> {
    let reader = FileReader::new().unwrap();

    // 파일을 Blob으로 변환
    let blob: Blob = file.slice().map_err(|e| {
        web_sys::console::error_1(&format!("Error slicing file: {:?}", e).into());
        e
    })?;

    // FileReader로 텍스트 읽기
    reader.read_as_text(&blob).map_err(|e| {
        web_sys::console::error_1(&format!("Error reading file: {:?}", e).into());
        e
    })?;

    // Promise를 생성하여 `onload`가 완료될 때까지 기다림
    let promise = Promise::new(&mut |resolve, _| {
        let onload_closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            resolve.call0(&JsValue::null()).unwrap();
        }) as Box<dyn FnMut(_)>);

        reader.set_onload(Some(onload_closure.as_ref().unchecked_ref()));
        onload_closure.forget(); // Rust에서 GC로부터 해제 방지
    });

    // `onload`가 완료될 때까지 대기
    JsFuture::from(promise).await?;

    // 읽은 파일 내용을 가져오기
    let svg_data= reader.result().unwrap().as_string().unwrap();
    web_sys::console::log_1(&format!("File content: {}", svg_data).into());

    let mut shapes = Vec::new();

    let parser = DomParser::new().unwrap();
    let doc = parser.parse_from_string(&svg_data, web_sys::SupportedType::ImageSvgXml).unwrap();

    if let Some(svg_element) = doc.query_selector("svg").ok().flatten() {
        //let gradients = self.extract_gradients(context, &svg_element);
        //self.extract_styles(&svg_element);
        match parse_svg_element(&svg_element) {
            Ok(elements) => shapes.extend(elements),
            Err(e) => web_sys::console::error_1(&format!("Error parsing SVG element: {:?}", e).into()),
        }
    } else {
        web_sys::console::log_1(&"⚠️ SVG 파싱 실패".into());
    }

    //render_svg_to_canvas(context, &canvas, &svg_data, drop_x, drop_y);

    Ok(shapes)
}

/// svg data를 파싱하여 svg element를 반환한다.
pub fn parse_svg_data(svg_data: &str) -> Result<Vec<Box<dyn DrawShape>>, JsValue> {
    let mut shapes = Vec::new();

    let parser = DomParser::new().unwrap();
    let doc = parser.parse_from_string(svg_data, web_sys::SupportedType::ImageSvgXml).unwrap();

    if let Some(svg_element) = doc.query_selector("svg").ok().flatten() {
        //let gradients = self.extract_gradients(context, &svg_element);
        //self.extract_styles(&svg_element);
        match parse_svg_element(&svg_element) {
            Ok(elements) => shapes.extend(elements),
            Err(e) => web_sys::console::error_1(&format!("Error parsing SVG element: {:?}", e).into()),
        }
    } else {
        web_sys::console::log_1(&"⚠️ SVG 파싱 실패".into());
    }

    //render_svg_to_canvas(context, &canvas, &svg_data, drop_x, drop_y);

    Ok(shapes)
}

// 🎯 SVG를 Canvas에 순서대로 그리는 함수 (g 요소 포함)
fn parse_svg_element(parent_element: &Element) -> Result<Vec<Box<dyn DrawShape>>, JsValue>{
    let mut shapes: Vec<Box<dyn DrawShape>> = Vec::new();

    let child_nodes = parent_element.child_nodes();
    for i in 0..child_nodes.length() {
        if let Some(node) = child_nodes.item(i) {
            if let Some(element) = node.dyn_ref::<Element>() {
                let tag_name = element.tag_name().to_lowercase();
                //let fill_style = self.parse_fill_attribute(element, gradients);

                match tag_name.as_str() {
                    //"g" => self.render_group(&context, element, gradients, &fill_style),
                    "line" =>{
                        if let Some(line) = parse_line(element){
                            shapes.push(line);
                        }
                    }
                    "rect" => {
                        if let Some(rectangle) = parse_rect(element){
                            shapes.push(rectangle);
                        }
                    },
                    "polygon" => {
                        if let Some(polygon) = parse_polygon(element){
                            shapes.push(polygon);
                        }
                    },
                    "polyline" => {
                        if let Some(polyline) = parse_polyline(element){
                            shapes.push(polyline);
                        }
                    },
                    "ellipse" => {
                        if let Some(ellipse) = parse_ellipse(element){
                            shapes.push(ellipse);
                        }
                    },
                    //"circle" => self.render_circle(&context, element, gradients),
                    //"path" => self.render_path(&context, element, gradients, &fill_style),
                    //"text" => self.render_text(&context, element, gradients),
                    _ => (),
                }
            }
        }
    }

    Ok(shapes)
}

fn parse_svg_style(style: &str) -> HashMap<String, String> {
    let mut styles_map = HashMap::new();

    for rule in style.split(';') {
        let parts: Vec<&str> = rule.split(':').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            styles_map.insert(parts[0].to_string(), parts[1].to_string());
        }
    }

    styles_map
}

/// parse line element
fn parse_line(rect_element: &Element) -> Option<Box<dyn DrawShape>>{
    let x1 = rect_element.get_attribute("x1").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
    let y1 = rect_element.get_attribute("y1").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
    let x2 = rect_element.get_attribute("x2").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
    let y2 = rect_element.get_attribute("y2").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);

    let mut color = "#000000".to_string();
    let mut fill = "none".to_string();
    if let Some(style) = rect_element.get_attribute("style"){
        let style_map = parse_svg_style(&style);

        if let Some(stroke_value) = style_map.get("stroke") {
            color = stroke_value.clone();
        }

        if let Some(fill_value) = style_map.get("fill") {
            fill = fill_value.clone();
        }
    }

    let line = Line::new(Point2D::new(x1, y1), Point2D::new(x2, y2), color.to_string(), 1.0);
    Some(Box::new(line))
}

/// parse rectangle element
fn parse_rect(rect_element: &Element) -> Option<Box<dyn DrawShape>>{
    let x_pos = rect_element.get_attribute("x").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
    let y_pos = rect_element.get_attribute("y").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
    let width = rect_element.get_attribute("width").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
    let height = rect_element.get_attribute("height").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
    let rx = rect_element.get_attribute("rx").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
    let ry = rect_element.get_attribute("ry").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);

    let mut color = "#000000".to_string();
    let mut fill = "none".to_string();
    if let Some(style) = rect_element.get_attribute("style"){
        let style_map = parse_svg_style(&style);
        if let Some(fill_value) = style_map.get("fill") {
            fill = fill_value.clone();
        }

        if let Some(stroke_value) = style_map.get("stroke") {
            color = stroke_value.clone();
        }
    }

    let rectangle = Rectangle::new(Point2D::new(x_pos, y_pos), width, height, color.to_string(), 1.0, Some(fill.to_string()));
    Some(Box::new(rectangle))
}

/// parse polyline element
fn parse_polyline(polyline_element: &Element) -> Option<Box<dyn DrawShape>>{
    if let Some(points_str) = polyline_element.get_attribute("points") {
        let mut points: Vec<Point2D> = Vec::new();
        let points_vec: Vec<&str> = points_str.split_whitespace().collect();
        if points_vec.len() >= 2 {
            if let Some(first_point) = points_vec.get(0) {
                let coords: Vec<f64> = first_point.split(',')
                    .filter_map(|s| s.parse::<f64>().ok())
                    .collect();
                if coords.len() == 2 {
                    points.push(Point2D::new(coords[0], coords[1]));
                }
            }

            for point in points_vec.iter().skip(1) {
                let coords: Vec<f64> = point.split(',')
                    .filter_map(|s| s.parse::<f64>().ok())
                    .collect();
                if coords.len() == 2 {
                    points.push(Point2D::new(coords[0], coords[1]));
                }
            }

            let mut color = "#000000".to_string();
            let mut fill = "none".to_string();
            if let Some(style) = polyline_element.get_attribute("style"){
                let style_map = parse_svg_style(&style);
                if let Some(fill_value) = style_map.get("fill") {
                    fill = fill_value.clone();
                }

                if let Some(stroke_value) = style_map.get("stroke") {
                    color = stroke_value.clone();
                }
            }

            let polyline = Polyline::new(points, color, 1.0, Some(fill.to_string()));
            return Some(Box::new(polyline));
        }
    }

    None
}

/// parse polygon element
fn parse_polygon(polygon_element: &Element) -> Option<Box<dyn DrawShape>>{
    if let Some(points_str) = polygon_element.get_attribute("points") {
        let mut points: Vec<Point2D> = Vec::new();
        let points_vec: Vec<&str> = points_str.split_whitespace().collect();
        if points_vec.len() >= 2 {
            if let Some(first_point) = points_vec.get(0) {
                let coords: Vec<f64> = first_point.split(',')
                    .filter_map(|s| s.parse::<f64>().ok())
                    .collect();
                if coords.len() == 2 {
                    points.push(Point2D::new(coords[0], coords[1]));
                }
            }

            for point in points_vec.iter().skip(1) {
                let coords: Vec<f64> = point.split(',')
                    .filter_map(|s| s.parse::<f64>().ok())
                    .collect();
                if coords.len() == 2 {
                    points.push(Point2D::new(coords[0], coords[1]));
                }
            }
            points.push(points[0]);

            let mut color = "#000000".to_string();
            let mut fill = "none".to_string();
            if let Some(style) = polygon_element.get_attribute("style"){
                let style_map = parse_svg_style(&style);
                if let Some(fill_value) = style_map.get("fill") {
                    fill = fill_value.clone();
                }

                if let Some(stroke_value) = style_map.get("stroke") {
                    color = stroke_value.clone();
                }
            }

            let polyline = Polyline::new(points, color, 1.0, Some(fill.to_string()));
            return Some(Box::new(polyline));
        }
    }

    None
}

// 🎯 Ellipse 요소 처리
fn parse_ellipse(ellipse_element: &Element) -> Option<Box<dyn DrawShape>>{
    let cx = ellipse_element.get_attribute("cx").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
    let cy = ellipse_element.get_attribute("cy").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
    let rx = ellipse_element.get_attribute("rx").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);
    let ry = ellipse_element.get_attribute("ry").unwrap_or("0".to_string()).parse::<f64>().unwrap_or(0.0);

    let mut color = "#000000".to_string();
    let mut fill = "none".to_string();
    if let Some(style) = ellipse_element.get_attribute("style"){
        let style_map = parse_svg_style(&style);
        if let Some(fill_value) = style_map.get("fill") {
            fill = fill_value.clone();
        }

        if let Some(stroke_value) = style_map.get("stroke") {
            color = stroke_value.clone();
        }
    }else if let Some(stroke) = ellipse_element.get_attribute("stroke"){
        color = stroke.clone();
    }

    let ellipse = Ellipse::new(Point2D::new(cx, cy), rx, ry, 0.0, 0.0, 2.0 * PI, color, 1.0, Some(fill));
    Some(Box::new(ellipse))
}

// 🎯 Canvas에 SVG를 벡터로 렌더링
pub fn render_svg_to_canvas(context: &mut WebRenderContext, _canvas: &Element, svg_data: &str, x: f64, y: f64) {
    let svg = Svg::new(Point2D::new(x, y), svg_data); 
    svg.draw(context, 1.0);

    let instance = VecDrawDoc::instance();
    instance.lock().unwrap().add_shape(Box::new(svg));
}