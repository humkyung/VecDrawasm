use std::any::Any;
use std::collections::HashMap;
use std::f64::MAX;
use std::iter::Scan;
use std::str;
use std::task::Context;
use std::thread::panicking;
use log::info;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_sys::console::group;
use web_sys::console::info;
use web_sys::{window, Document, CanvasRenderingContext2d, MouseEvent, CompositionEvent, InputEvent, KeyboardEvent};

use super::geometry::Vector2D;
use super::geometry::{Point2D};
use super::line;
use super::shape::{Shape};

pub struct TextBoxManager {
    document: Document,
    context: CanvasRenderingContext2d,
    boxes: Vec<TextBox>,
    active_index: Option<usize>,
    cursor_visible: bool,
    is_composing: bool,
    composition_text: String,
}
impl TextBoxManager {
    pub fn new(document: Document, context: CanvasRenderingContext2d) -> Self {
        Self {
            document,
            context,
            boxes: Vec::new(),
            active_index: None,
            cursor_visible: true,
            is_composing: false,
            composition_text: String::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.active_index.is_some()
    }

    pub fn on_click(&mut self, event: MouseEvent, current_x: f64, current_y: f64) {
        let x = event.client_x() as f64;
        let y = event.client_y() as f64;

        // 기존 박스 클릭 시 해당 박스를 활성화
        for (i, box_) in self.boxes.iter_mut().enumerate() {
            if box_.contains(x, y) {
                self.active_index = Some(i);
                self.focus_hidden_input();
                self.redraw();
                return;
            }
        }

        // 새 박스 생성
        self.boxes.push(TextBox::new(current_x, current_y));
        self.active_index = Some(self.boxes.len() - 1);

        self.focus_hidden_input();
        self.redraw();
    }

    pub fn finish_input(&mut self) {
        // 입력 완료 및 비활성화
        self.active_index = None;
        self.clear_hidden_input();
        self.redraw();
    }

    pub fn on_composition_start(&mut self) {
        self.is_composing = true;
        self.composition_text.clear();
    }

    /// 글자 조합 중인 상태
    pub fn on_composition_update(&mut self, event: CompositionEvent) {
        if let Some(index) = self.active_index {
            let active_box = &mut self.boxes[index];
            self.composition_text = event.data().unwrap();
            active_box.composition_text = self.composition_text.clone();
            self.redraw();
        }
    }

    /// 글자 조합이 완료된 상태
    pub fn on_composition_end(&mut self, event: CompositionEvent) {
        self.is_composing = false;
        if let Some(index) = self.active_index {
            let active_box = &mut self.boxes[index];
            if let Some(data) = event.data() {
                let index = active_box.get_byte_index_at_cursor();
                active_box.insert_at_cursor(&data, index);
                active_box.composition_text.clear();
            }
            self.redraw();
        }
        self.clear_hidden_input();
    }

    pub fn on_input(&mut self, event: InputEvent) {
        if self.is_composing {
            return; // IME 조합 중에는 input 이벤트 무시
        }

        if let Some(index) = self.active_index {
            let active_box = &mut self.boxes[index];
            let value = event.data().unwrap_or_default();

            let cursor_pos = active_box.get_byte_index_at_cursor();
            active_box.insert_at_cursor(&value, cursor_pos);

            // 텍스트의 너비 계산 및 업데이트
            let text_clone = active_box.text.clone();
            let text_width = {
                let text_clone = active_box.text.clone();
                get_text_width(&self.context, &text_clone)
            };
            active_box.update_width(text_width);

            self.clear_hidden_input();
            self.redraw();
        }
    }

    pub fn on_keydown(&mut self, event: KeyboardEvent) {
        if self.is_composing {
            return; // IME 조합 중에는 keydown 이벤트 무시
        }

        if let Some(index) = self.active_index {
            let active_box = &mut self.boxes[index];
            let text_clone = active_box.text.clone();

            match event.key().as_str() {
                "Backspace" => {
                    active_box.delete_before_cursor();
                    let text_width = get_text_width(&self.context, &text_clone);
                    active_box.update_width(text_width);
                    self.redraw();
                }
                "Delete" => {
                    active_box.delete_at_cursor();
                    let text_width = get_text_width(&self.context, &text_clone);
                    active_box.update_width(text_width);
                    self.redraw();
                }
                "Enter" => {
                    let cursor_pos = active_box.get_char_index_at_cursor();
                    active_box.insert_at_cursor("\n", cursor_pos);
                    info!("Enter: {},{}", active_box.text,active_box.cursor_position);

                    // ✅ TextBox 높이 증가 (줄 개수에 맞게)
                    active_box.update_height(get_text_height(&active_box.text));
                    active_box.update_width(get_max_line_width(&self.context, &active_box.text));
                    
                    self.redraw();
                }
                "ArrowLeft" => {
                    active_box.move_cursor_left();
                    self.redraw();
                }
                "ArrowRight" => {
                    active_box.move_cursor_right();
                    self.redraw();
                }
                "ArrowUp" =>{
                    active_box.move_cursor_up();
                    self.redraw();
                }
                "ArrowDown" =>{
                    active_box.move_cursor_down();
                    self.redraw();
                }
                "Home" =>{
                    active_box.cursor_position = 0;
                    self.redraw();
                }
                "End" =>{
                    active_box.cursor_position = active_box.text.chars().count();
                    self.redraw();
                }
                _ => {}
            }
        }
    }

    fn focus_hidden_input(&self) {
        let textarea = self.document.get_element_by_id("hidden-input").unwrap();
        let input = textarea.dyn_ref::<web_sys::HtmlTextAreaElement>().unwrap();
        input.focus().unwrap();
    }

    fn clear_hidden_input(&self) {
        let textarea = self.document.get_element_by_id("hidden-input").unwrap();
        let input = textarea.dyn_ref::<web_sys::HtmlTextAreaElement>().unwrap();
        input.set_value("");
    }

    pub fn toggle_cursor(&mut self) {
        self.cursor_visible = !self.cursor_visible;
        self.redraw();
    }

    fn redraw(&self) {
        //self.context.clear_rect(0.0, 0.0, 800.0, 600.0);
        self.context.set_font("20px sans-serif");

        for (i, box_) in self.boxes.iter().enumerate() {
            self.context.set_fill_style(&"lightgray".into());
            self.context.fill_rect(box_.position.x, box_.position.y, box_.width, box_.height);

            self.context.set_fill_style(&"black".into());

            let byte_index = box_.get_byte_index_at_cursor();
            let text_before_cursor = &box_.text[..byte_index];
            let text_after_cursor = &box_.text[byte_index..];
            let text_to_draw = format!("{}{}{}", text_before_cursor, box_.composition_text, text_after_cursor);
            let lines: Vec<&str> = text_to_draw.lines().collect();
            for(line_idx, line) in lines.iter().enumerate(){
                self.context
                    .fill_text(line, box_.position.x + 5.0, box_.position.y + 20.0 + (line_idx as f64) * 30.0)
                    .unwrap();
            }

            // 커서 및 조합 중인 글자 강조 표시
            if self.active_index == Some(i) {
                let cursor_x = get_text_width(&self.context, &text_to_draw[..box_.get_byte_index_at_cursor()]) + box_.position.x + 5.0;
                let cursor_y = box_.position.y + 5.0 + (text_to_draw[..box_.get_byte_index_at_cursor()].matches('\n').count() as f64) * 30.0;
                if self.is_composing && !box_.composition_text.is_empty() {
                    // 조합 중인 글자에 파란색 박스 표시
                    let width = get_text_width(&self.context, &box_.composition_text);
                    self.context.set_stroke_style(&"blue".into());
                    self.context.stroke_rect(cursor_x, cursor_y, width, 22.0);
                }else if self.cursor_visible {
                    self.context.set_fill_style(&"blue".into());
                    self.context.fill_rect(cursor_x, cursor_y, 2.0, 20.0);
                }

                self.context.set_stroke_style(&"blue".into());
            } else {
                self.context.set_stroke_style(&"gray".into());
            }
            self.context.stroke_rect(box_.position.x, box_.position.y, box_.width, box_.height);
        }
    }
}

fn get_text_height(text: &str) -> f64 {
    let line_count = text.lines().count().max(1); // 최소 1줄
    (line_count as f64) * 30.0 // ✅ 줄 개수 × 줄 높이
}

fn get_max_line_width(context: &CanvasRenderingContext2d, text: &str) -> f64 {
    text.lines()
        .map(|line| get_text_width(context,line))
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(50.0) // 최소 너비 50px
}

/// 텍스트 마지막 줄의 길이를 반환한다.
fn get_text_width(context: &CanvasRenderingContext2d, text: &str) -> f64 {
    // 마지막 줄이 개행 문자로 끝나면 길이 0
    if text.ends_with('\n'){
        return 0.0;
    }

    let last_text = text.lines().last().unwrap_or_default();
    context
        .measure_text(last_text)
        .map(|metrics| metrics.width())
        .unwrap_or_else(|_| 0.0)
}

#[derive(Debug, Clone)]
pub struct TextBox{
    pub position: Point2D,
    pub text: String,
    rotation: f64,
    selected: bool,
    hovered: bool,
    color: String,
    axis_x: Vector2D,
    axis_y: Vector2D,
    pub width: f64,
    height: f64,
    pub composition_text: String,
    pub cursor_position: usize,
}
impl TextBox{
    pub fn new(x: f64, y: f64) -> Self {
        TextBox{
            position: Point2D::new(x, y)
            , text: String::new()
            , rotation: 0.0
            , selected: false
            , hovered: false
            , color: "#000000".to_string()
            , axis_x: Vector2D::AXIS_X
            , axis_y: Vector2D::AXIS_Y,
            width: 50.0,
            height: 30.0,
            composition_text: String::new(),
            cursor_position: 0,}
    }

    fn control_points(&self) -> Vec<Point2D>{
        let control_pts = vec![
            Point2D::new(self.position.x, self.position.y) ,
            Point2D::new(self.position.x, self.position.y - 30.0)
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

    pub fn contains(&self, mouse_x: f64, mouse_y: f64) -> bool {
        mouse_x >= self.position.x && mouse_x <= self.position.x + self.width && mouse_y >= self.position.y && mouse_y <= self.position.y + 30.0
    }

    /// ✅ 줄 개수에 따라 높이 자동 조정
    fn update_height(&mut self, height: f64) {
        self.height = height;
    }

    /// ✅ 현재 줄의 끝 위치로 커서 이동
    fn move_cursor_to_line_end(&mut self) {
        if let Some(next_newline) = self.text[self.cursor_position..].find('\n') {
            self.cursor_position += next_newline;
        } else {
            self.cursor_position = self.text.len();
        }
    }

    /// ✅ 위쪽 줄로 이동
    fn move_cursor_up(&mut self) {
        if self.cursor_position == 0 {
            return; // 🚫 첫 줄에서는 더 위로 이동할 수 없음
        }

        let mut pos = self.cursor_position;
        let mut row = self.get_row_index_at_cursor();
        let mut col = self.get_column_index_at_cursor();
        info!("mouse move up row: {}, col: {}", row, col);

        if row > 0{
            row -= 1;

            let lines = self.text.lines();
            if let Some(line) = lines.clone().nth(row) {
                if line.chars().count() < col {
                    col = line.chars().count();
                }
            }

            info!("row: {}, col: {}", row, col);
            pos = lines.take(row - 1).map(|line| line.chars().count()).sum::<usize>() + col;
        }

        self.cursor_position = pos;

        let index = self.get_byte_index_at_cursor();
        info!("cursor line: {},{}", index, &self.text[..index]);
    }

    /// ✅ 아래쪽 줄로 이동
    fn move_cursor_down(&mut self) {

        let mut pos = self.cursor_position;
        let mut row = self.get_row_index_at_cursor();
        let mut col = self.get_column_index_at_cursor();

        info!("row: {}, col: {}", row, col);

        let lines = self.text.lines();
        let line_count = lines.clone().count();
        if row < line_count{
            row += 1;

            if let Some(line) = lines.clone().nth(row) {
                if line.chars().count() < col {
                    col = line.chars().count();
                }

                let row_chars = lines.take(row - 1).map(|line| line.chars().count()).sum::<usize>();
                info!("row_chars: {}, col: {}", row_chars, col);
                pos = row_chars + col;
            }
        }

        self.cursor_position = pos;
    }

    pub fn get_char_index_at_cursor(&self) -> usize {
        self.cursor_position
    }

    pub fn get_byte_index_at_cursor(&self) -> usize {
        let index = self.text
            .char_indices()
            .nth(self.cursor_position)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len());
        index
    }

    /// 커서 위치에 텍스트를 입력한다.
    pub fn insert_at_cursor(&mut self, value: &str, index: usize) {
        let byte_index = self.get_byte_index_at_cursor();
        self.text.insert_str(byte_index, value);
        self.cursor_position += value.chars().count();
    }

    /// 커서 위치의 이전 글자를 삭제한다.
    pub fn delete_before_cursor(&mut self) {
        if self.cursor_position > 0 {
            let byte_index = self.get_byte_index_at_cursor();
            let prev_char_index = self
                .text
                .char_indices()
                .take(self.cursor_position)
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);

            self.text.replace_range(prev_char_index..byte_index, "");
            self.cursor_position -= 1;
        }
    }

    pub fn delete_at_cursor(&mut self) {
        if self.cursor_position < self.text.chars().count() {
            let start_index = self.get_byte_index_at_cursor();
            let end_index = self.text[start_index..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| start_index + i)
                .unwrap_or(0);

            self.text.replace_range(start_index..end_index, "");
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.text.chars().count() {
            self.cursor_position += 1;
        }
    }

    pub fn get_row_index_at_cursor(&self) -> usize {
        let index = self.get_byte_index_at_cursor();
        let row = self.text[..index].lines().count();
        row - 1
    }

    pub fn get_column_index_at_cursor(&self) -> usize {
        let index = self.get_byte_index_at_cursor();
        let lines = self.text[..index].lines();
        let column = lines.last().map_or(0, |line| line.chars().count());
        column - 1
    }

    pub fn update_width(&mut self, text_width: f64) {
        // 최소 50px, 최대 400px
        self.width = text_width.clamp(50.0, 400.0) + 10.0; // padding 포함
    }
}
impl Shape for TextBox{
    fn color(&self) -> &str {
        &self.color
    }

    fn line_width(&self) -> f64 {
        0.0
    }

    fn max_point(&self) -> Point2D{
        Point2D::new(self.position.x, self.position.y)
    }

    fn min_point(&self) -> Point2D{
        Point2D::new(self.position.x, self.position.y)
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

    fn get_control_point(&self, x: f64, y: f64, scale: f64) -> i32{
        let mut control_pts = self.control_points();
        for pt in &mut control_pts{
            let mut dir = Vector2D::from_points(self.position, *pt);
            dir.rotate_by(self.rotation);
            pt.x = self.position.x + dir.x;
            pt.y = self.position.y + dir.y;
        }

        let adjusted_width = (10.0 / scale).powi(2);
        control_pts.iter().position(|p| (x - p.x).powi(2) + (y - p.y).powi(2) < adjusted_width).map_or(-1, |i| i as i32)
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
        self.position.x += dx;
        self.position.y += dy;
    }

    fn move_control_point_by(&mut self, index: i32, dx: f64, dy: f64) {
        let mut control_pts = self.control_points();
        for pt in &mut control_pts{
            let mut dir = Vector2D::from_points(self.position, *pt);
            dir.rotate_by(self.rotation);
            pt.x = self.position.x + dir.x;
            pt.y = self.position.y + dir.y;
        }

        if index == 0{
            self.position.x += dx;
            self.position.y += dy;
        }
    }

    fn draw(&mut self, context: &CanvasRenderingContext2d, scale: f64){
        context.save();

        context.translate(self.position.x, self.position.y).unwrap();
        context.rotate(self.rotation).unwrap();
        context.translate(-self.position.x, -self.position.y).unwrap();

        if self.hovered{
            context.set_stroke_style(&JsValue::from_str("#ff0000"));
        }
        else{
            context.set_stroke_style(&JsValue::from_str(&self.color));
        }

        context.set_fill_style(&"#000000".into()); // Black text
        context.set_font("20px Arial");
        context.fill_text(&self.text, self.position.x, self.position.y).unwrap();

        if self.selected{ self.draw_control_points(context, scale);}

        context.restore();
    }   

    fn draw_xor(&self, context: &CanvasRenderingContext2d, scale: f64){
        context.save();

        context.set_global_composite_operation("xor").unwrap();

        context.translate(self.position.x, self.position.y).unwrap();
        context.rotate(self.rotation).unwrap();
        context.translate(-self.position.x, -self.position.y).unwrap();

        if self.hovered{
            context.set_stroke_style(&JsValue::from_str("#ff0000"));
        }
        else{
            context.set_stroke_style(&JsValue::from_str(&self.color));
        }

        context.set_fill_style(&"#000000".into()); // Black text
        context.set_font("20px Arial");
        context.fill_text(&self.text, self.position.x, self.position.y).unwrap();

        context.restore();
    }

    fn draw_control_points(&self, context: &CanvasRenderingContext2d, scale: f64) {
        let adjusted_width = 5.0 / scale;

        context.save();

        let control_pts = self.control_points();
        context.set_fill_style(&"#29B6F2".into()); // Red control points
        for point in control_pts{
            context.fill_rect(point.x - adjusted_width, point.y - adjusted_width, adjusted_width * 2.0, adjusted_width * 2.0);
        }

        context.set_stroke_style(&"#29B6F2".into()); // blue line
        let adjusted_width = 0.5 / scale;
        context.set_line_width(adjusted_width);

        // ✅ Set dash pattern: [Dash length, Gap length]
        let dash_pattern = js_sys::Array::new();
        dash_pattern.push(&(adjusted_width * 3.0).into());  // dash
        dash_pattern.push(&(adjusted_width * 3.0).into());  // gap
        context.set_line_dash(&dash_pattern).unwrap();

        //context.begin_path();
        //context.rect(self.center.x - self.radius_x, self.center.y - self.radius_y, self.radius_x * 2.0, self.radius_y * 2.0);
        //context.stroke();

        context.restore();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}