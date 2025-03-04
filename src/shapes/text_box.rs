use std::any::Any;
use std::collections::HashMap;
use std::f64::MAX;
use std::iter::Scan;
use std::str;
use std::task::Context;
use std::thread::panicking;
use std::rc::Rc;
use std::cell::RefCell;
use js_sys::Reflect::get;
use log::info;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use piet::{RenderContext, Color, Text, TextLayout, TextLayoutBuilder, FontFamily, StrokeStyle};
use kurbo::Affine;
use piet_web::WebRenderContext;

use web_sys::console::group;
use web_sys::console::info;
use web_sys::{window, HtmlCanvasElement, Document, CanvasRenderingContext2d, MouseEvent, CompositionEvent, InputEvent, KeyboardEvent};

use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use std::thread;

use crate::state::State;
use super::geometry::{Point2D, Vector2D, BoundingRect2D};
use super::shape::{Shape, convert_to_color};

pub struct TextBoxManager {
    document: Document,
    context: Rc<RefCell<WebRenderContext<'static>>>,
    attached: Option<Arc<Mutex<Box<dyn Shape>>>>,
    cursor_visible: bool,
    is_composing: bool,
    composition_text: String,
}
unsafe impl Send for TextBoxManager{}
unsafe impl Sync for TextBoxManager{}

impl TextBoxManager {
    pub fn new(document: Document, context: Rc<RefCell<WebRenderContext<'static>>>) -> Self {
        Self {
            document,
            context,
            attached: None,
            cursor_visible: true,
            is_composing: false,
            composition_text: String::new(),
        }
    }

    pub fn instance() -> Arc<Mutex<Self>> {
        static INSTANCE: Lazy<Arc<Mutex<TextBoxManager>>> = Lazy::new(|| {
            let window = web_sys::window().expect("No global window exists");
            let document = window.document().expect("No document found");
            let canvas = document
                .get_element_by_id("drawing-canvas")
                .expect("Canvas element not found")
                .dyn_into::<HtmlCanvasElement>()
                .expect("Failed to cast to HtmlCanvasElement");
            // Get rendering context
            let context = match canvas.get_context("2d") {
                Ok(Some(ctx)) => ctx.dyn_into::<web_sys::CanvasRenderingContext2d>().unwrap(),
                Ok(None) => panic!("Failed to get 2D context"),
                Err(_) => panic!("Failed to get 2D context"),
            };

            let context = Rc::new(RefCell::new(WebRenderContext::new(context.clone(), window.clone())));
            Arc::new(Mutex::new(TextBoxManager::new(document, context)))
        });
        Arc::clone(&INSTANCE) // ✅ 공유된 인스턴스 반환
    }

    /// 텍스트 박스를 연결한다.
    pub fn attach(&mut self, attached: Arc<Mutex<Box<dyn Shape>>>, state: &State) {
        self.attached = Some(attached);

        self.focus_hidden_input();
        self.redraw(state);
    }

    pub fn detach(&mut self) {
        self.attached = None;
    }

    pub fn is_active(&self) -> bool {
        self.attached.is_some()
        //self.attached.as_ref().map(|a| a.lock().unwrap()).is_some()
    }

    /// 마우스 클릭 이벤트 처리
    pub fn on_click(&mut self, context: &mut WebRenderContext, event: MouseEvent, current_x: f64, current_y: f64, scale: f64, state: &State){
        // 기존 박스 클릭 시 해당 박스를 활성화
        /*
        for (i, box_) in self.boxes.iter_mut().enumerate() {
            if box_.is_hit(current_x, current_y, scale) {
                self.active_index = Some(i);
                self.focus_hidden_input();
                self.redraw();
                return;
            }
        }
        */

        // 새 박스 생성
        //self.set_attached(Some(Box::new(TextBox::new(current_x, current_y))));

        self.focus_hidden_input();
        self.redraw(state);
    }

    pub fn finish_input(&mut self, state: &State) {
        // 입력 완료 및 비활성화
        self.detach();
        self.clear_hidden_input();
        self.redraw(state);
    }

    /// 글자 조합 시작
    pub fn on_composition_start(&mut self) {
        self.is_composing = true;
        self.composition_text.clear();
    }

    /// 글자 조합 중인 상태
    pub fn on_composition_update(&mut self, event: CompositionEvent, state: &State) {
        if !self.is_active() {
            return;
        }

        if let Some(attached) = &self.attached {
            let mut shape = attached.lock().unwrap();
            if let Some(tb) = shape.as_any_mut().downcast_mut::<TextBox>() {
                self.composition_text = event.data().unwrap();
                tb.composition_text = self.composition_text.clone();
                drop(tb); // Release the mutable borrow before calling redraw
            }
            drop(shape); // Release the immutable borrow before calling redraw
            self.redraw(state);
        }
    }

    /// 글자 조합이 완료된 상태
    pub fn on_composition_end(&mut self, event: CompositionEvent, state: &State) {
        self.is_composing = false;

        if !self.is_active() {
            return;
        }

        if let Some(attached) = &self.attached {
            let mut shape = attached.lock().unwrap();
            if let Some(tb) = shape.as_any_mut().downcast_mut::<TextBox>() {
                if let Some(data) = event.data() {
                    tb.insert_at_cursor(&data);
                    info!("on_composition_end: {}", tb.text);

                    // 텍스트의 너비 계산 및 업데이트
                    let text_width = {
                        let text_clone = tb.text.clone();
                        get_max_line_width(&mut self.context.borrow_mut(), &text_clone)
                    };
                    tb.update_width(text_width);
                    tb.composition_text.clear();
                }
                drop(shape); // Release the mutable borrow before calling redraw
                self.redraw(state);
            }
        }

        self.clear_hidden_input();
    }

    pub fn on_input(&mut self, event: InputEvent, state: &State) {
        if self.is_composing || !self.is_active(){
            return; // IME 조합 중에는 input 이벤트 무시
        }

        if let Some(attached) = &self.attached {
            let mut shape = attached.lock().unwrap();
            if let Some(tb) = shape.as_any_mut().downcast_mut::<TextBox>() {
                let value = event.data().unwrap_or_default();

                info!("on_input: {}", value);
                tb.insert_at_cursor(&value);

                // 텍스트의 너비 계산 및 업데이트
                let text_width = {
                    let text_clone = tb.text.clone();
                    get_max_line_width(&mut self.context.borrow_mut(), &text_clone)
                };
                info!("text_width {:?}", text_width);
                tb.update_width(text_width);
                drop(tb); // Release the mutable borrow before calling redraw

                self.clear_hidden_input();
            }

            drop(shape); // Release the immutable borrow before calling redraw
            self.redraw(state);
        }
    }

    /// 키보드 입력 처리
    pub fn on_keydown(&mut self, event: KeyboardEvent, state: &State) {
        if self.is_composing || !self.is_active() {
            return; // IME 조합 중에는 keydown 이벤트 무시
        }

        if let Some(attached) = &self.attached {
            let mut shape = attached.lock().unwrap();
            if let Some(tb) = shape.as_any_mut().downcast_mut::<TextBox>() {
                let text_clone = tb.text.clone();
                match event.key().as_str() {
                    "Backspace" => {
                        tb.delete_before_cursor();
                        let text_width = get_max_line_width(&mut self.context.borrow_mut(), &text_clone);
                        tb.update_width(text_width);
                        let height = tb.get_height(&tb.text);
                        tb.update_height(height);
                    }
                    "Delete" => {
                        tb.delete_at_cursor();
                        let text_width = get_max_line_width(&mut self.context.borrow_mut(), &text_clone);
                        tb.update_width(text_width);
                        let height = tb.get_height(&tb.text);
                        tb.update_height(height);
                    }
                    "Enter" => {
                        tb.insert_at_cursor("\n");

                        // ✅ TextBox 높이 증가 (줄 개수에 맞게)
                        let height = tb.get_height(&tb.text);
                        tb.update_height(height);
                        let max_line_width = get_max_line_width(&mut self.context.borrow_mut(), &tb.text);
                        tb.update_width(max_line_width);
                    }
                    "ArrowLeft" => {
                        tb.move_cursor_left();
                    }
                    "ArrowRight" => {
                        tb.move_cursor_right();
                    }
                    "ArrowUp" =>{
                        tb.move_cursor_up();
                    }
                    "ArrowDown" =>{
                        tb.move_cursor_down();
                    }
                    "Home" =>{
                        tb.move_cursor_to_line_start();
                    }
                    "End" =>{
                        tb.move_cursor_to_line_end();
                    }
                    "Escape" => {
                        drop(tb); // Release the mutable borrow before calling finish_input
                        drop(shape); // Release the mutable borrow before calling finish_input
                        self.finish_input(state);
                        return;
                    }
                    _ => {}
                }

                drop(tb); // Release the mutable borrow before calling redraw 
            }

            drop(shape);
            self.redraw(state);
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

    /// 커서 표시 여부를 토글한다.
    pub fn toggle_cursor(&mut self, state: &State) {
        if !self.is_active() {
            return;
        }
        self.cursor_visible = !self.cursor_visible;
        self.redraw(state);
    }

    fn redraw(&mut self, state: &State) {
        if !self.is_active() {
            return;
        }

        if let Some(attached) = &self.attached {
            let mut shape = attached.lock().unwrap();
            if let Some(tb) = shape.as_any_mut().downcast_mut::<TextBox>() {
                
                let mut render_context = self.context.borrow_mut();
                let _ = render_context.save();
                let _ = render_context.transform(Affine::new([state.scale(), 0.0, 0.0, state.scale(), state.offset().x, state.offset().y]));

                let byte_index = tb.get_byte_index_at_cursor();
                let text_before_cursor = &tb.text[..byte_index];
                let text_after_cursor = &tb.text[byte_index..];
                let text_to_draw = format!("{}{}{}", text_before_cursor, tb.composition_text, text_after_cursor);
                info!("redraw: {}", text_to_draw);
                tb.update_width(get_max_line_width(&mut *render_context, &text_to_draw));

                tb.draw(&mut *render_context, state.scale());

                // 커서 및 조합 중인 글자 강조 표시
                let cursor_x = get_last_line_text_width(&mut *render_context, &text_to_draw[..tb.get_byte_index_at_cursor()]) + tb.position.x + 5.0;
                let cursor_y = tb.position.y + 5.0 + (text_to_draw[..tb.get_byte_index_at_cursor()].matches('\n').count() as f64) * (tb.get_font_size() + tb.get_line_gap());
                if self.is_composing && !tb.composition_text.is_empty() {
                    // 조합 중인 글자에 파란색 박스 표시
                    let cursor_width = get_last_line_text_width(&mut *render_context, &tb.composition_text);
                    let cursor_height = tb.get_font_size() + tb.get_line_gap();
                    let color = Color::rgba(0.0, 0.0, 255.0, 0.3);// ✅ 반투명한 파란색 (alpha = 0.3)
                    let rect = piet::kurbo::Rect::new(cursor_x, cursor_y, cursor_x + cursor_width, cursor_y + cursor_height);
                    render_context.fill(rect, &color);//cursor_x, cursor_y, cursor_width, cursor_height);
                }else if self.cursor_visible {
                    let color = Color::rgba(0.0, 0.0, 255.0, 0.3);// ✅ 반투명한 파란색 (alpha = 0.3)
                    let rect = piet::kurbo::Rect::new(cursor_x, cursor_y, cursor_x + 2.0, cursor_y + tb.get_font_size() + tb.get_line_gap());
                    render_context.fill(rect, &color);//cursor_x, cursor_y, cursor_width, cursor_height);
                }

                let _ = render_context.restore();
            }
        }
    }
}

/// 주어진 text를 줄 단위로 분할하여 반환한다.
/// '\n'으로 끝나는 경우 마지막 줄을 추가한다.
fn split_lines<'a>(text: &'a str) -> Vec<&'a str> {
    if text.is_empty() {
        return vec![""];
    }

    let mut lines: Vec<&'a str> = text.lines().collect();
    if text.ends_with('\n') {
        lines.push("");
    }
    lines
}

fn get_max_line_width(context: &mut WebRenderContext, text: &str) -> f64 {
    text.lines()
        .map(|line| get_last_line_text_width(context,line))
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(50.0) // 최소 너비 50px
}

/// 문자의 높이를 반환한다.
fn get_char_height(context: &CanvasRenderingContext2d, text: &str) -> f64 {
    let metrics = context.measure_text(text).unwrap();
    metrics.actual_bounding_box_ascent() + metrics.actual_bounding_box_descent() // ✅ 글자 높이 반환
}

// 제일 마지막 줄의 길이를 반환한다.
fn get_last_line_text_width(context: &mut WebRenderContext, text: &str) -> f64 {
    // 마지막 줄이 개행 문자로 끝나면 길이 0
    if text.ends_with('\n'){
        return 0.0;
    }

    let last_text = text.lines().last().unwrap_or_default();
    let layout = context.text().new_text_layout(last_text.to_string())
        .default_attribute(piet::TextAttribute::FontFamily(FontFamily::SERIF))
        .default_attribute(piet::TextAttribute::FontSize(20.0))
        .build()
        .unwrap();

    let width = layout.trailing_whitespace_width();
    width
}

#[derive(Debug, Clone)]
pub struct TextBox{
    pub position: Point2D,
    pub text: String,
    rotation: f64,
    selected: bool,
    hovered: bool,
    color: String,
    background_color: String,
    axis_x: Vector2D,
    axis_y: Vector2D,
    font_size: f64,
    line_gap: f64,
    pub width: f64,
    height: f64,
    selected_control_point: i32,
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
            , background_color: "lightgray".to_string() //
            , axis_x: Vector2D::AXIS_X
            , axis_y: Vector2D::AXIS_Y
            , font_size: 20.0
            , line_gap: 5.0
            , width: 50.0
            , height: 30.0
            , selected_control_point: -1
            , composition_text: String::new()
            , cursor_position: 0}
    }

    fn control_points(&self) -> Vec<Point2D>{
        let control_pts = vec![
            Point2D::new(self.position.x + self.width * 0.5, self.position.y + self.height * 0.5) ,
            Point2D::new(self.position.x + self.width * 0.5, self.position.y - 30.0)
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

    pub fn get_font_size(&self) -> f64 {
        self.font_size
    }

    pub  fn get_line_gap(&self) -> f64 {
        self.line_gap
    }

    // ✅ 텍스트 박스 높이 계산
    fn get_height(&self, text: &str) -> f64 {
        let lines = split_lines(text);
        let line_count = lines.into_iter().count().max(1); // 최소 1줄
        10.0 + (line_count as f64) * (self.font_size) + ((line_count - 1) as f64) * (self.line_gap)
    }

    /// ✅ 줄 개수에 따라 높이 자동 조정
    fn update_height(&mut self, height: f64) {
        self.height = height;
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

            info!("cursor_position: {}, row: {}, col: {}", self.cursor_position, row, col);
            pos = lines.take(row).map(|line| line.chars().count() + 1).sum::<usize>() + col;
            info!("pos: {}", pos);
        }

        self.cursor_position = pos;
    }

    /// ✅ 아래쪽 줄로 이동
    fn move_cursor_down(&mut self) {

        let mut pos = self.cursor_position;
        let mut row = self.get_row_index_at_cursor();
        let mut col = self.get_column_index_at_cursor();

        let lines = split_lines(&self.text);
        let line_count = lines.iter().count();

        info!("mouse move down cursor_position: {}, row: {}, col: {}, count: {}", self.cursor_position, row, col, line_count);

        if row < line_count - 1{
            row += 1;

            let line = lines[row];
            info!("line: {}", line);
            if line.chars().count() < col {
                col = line.chars().count();
            }

            let row_chars = lines.into_iter().take(row).map(|line| line.chars().count() + 1).sum::<usize>();
            info!("row_chars: {}, col: {}", row_chars, col);
            pos = row_chars + col;
        }

        self.cursor_position = pos;
    }

    /// ✅ 현재 줄의 시작으로 이동
    fn move_cursor_to_line_start(&mut self) {
        let row = self.get_row_index_at_cursor();
        let col = 0;

        let lines = split_lines(&self.text);

        let row_chars = lines.into_iter().take(row).map(|line| line.chars().count() + 1).sum::<usize>();
        let pos = row_chars + col;

        self.cursor_position = pos;
    }

    /// ✅ 현재 줄의 끝으로 이동
    fn move_cursor_to_line_end(&mut self) {
        let mut pos = self.cursor_position;
        let mut row = self.get_row_index_at_cursor();
        let mut col = self.get_column_index_at_cursor();

        let lines = split_lines(&self.text);
        let line = lines[row];
        col = line.chars().count();

        let row_chars = lines.into_iter().take(row).map(|line| line.chars().count() + 1).sum::<usize>();
        pos = row_chars + col;

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
    pub fn insert_at_cursor(&mut self, value: &str) {
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
        info!("move_cursor_left: {}, {}", self.cursor_position, self.text);
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.text.chars().count() {
            self.cursor_position += 1;
        }
    }

    pub fn get_row_index_at_cursor(&self) -> usize {
        let index = self.get_byte_index_at_cursor();
        let line = &self.text[..index];
        info!("get_row_index_at_cursor: {}, {}, {}", self.cursor_position, index, line);
        let row = split_lines(line).iter().count() - 1;
        info!("get_row_index_at_cursor: {}, {}", split_lines(line).join("\n"), row);
        row
    }

    pub fn get_column_index_at_cursor(&self) -> usize {
        let index = self.get_byte_index_at_cursor();
        let line = &self.text[..index];
        if line.is_empty() || line.ends_with('\n') {
            0
        } else {
            let column = line.lines().last().map_or(0, |line| line.chars().count());
            column
        }
    }

    pub fn update_width(&mut self, text_width: f64) {
        // 최소 50px
        self.width = text_width.clamp(50.0, f64::MAX) + 10.0; // padding 포함
    }
}
impl Shape for TextBox{
    fn color(&self) -> &str {
        &self.color
    }

    fn line_width(&self) -> f64 {
        0.0
    }

    fn min_point(&self) -> Point2D{
        Point2D::new(self.position.x, self.position.y)
    }

    fn max_point(&self) -> Point2D{
        Point2D::new(self.position.x + self.width, self.position.y + self.height)
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

    fn draw(&self, context: &mut WebRenderContext, scale: f64){
        let _ = context.save();

        context.transform(Affine::translate((self.position.x, self.position.y)));
        context.transform(Affine::rotate(self.rotation));
        context.transform(Affine::translate((-self.position.x, -self.position.y)));

        let mut color = convert_to_color(&self.color);
        if self.hovered{
            color = Color::RED;
        }

        //context.set_font("20px sans-serif");
        
        /// ✅ Draw text box
        let background= convert_to_color(&self.background_color);
        let rect = piet::kurbo::Rect::new(self.position.x, self.position.y, self.position.x + self.width, self.position.y + self.height + 5.0);
        context.fill(rect, &background);
        let rect = piet::kurbo::Rect::new(self.position.x, self.position.y, 
            self.position.x + self.width, self.position.y + self.height.max(self.font_size + self.line_gap) + 5.0);
        let adjusted_width = 1.0 / scale;
        context.stroke(rect, &color, adjusted_width);

        //context.set_fill_style(&self.color.as_str().into());

        let byte_index = self.get_byte_index_at_cursor();
        let text_before_cursor = &self.text[..byte_index];
        let text_after_cursor = &self.text[byte_index..];
        let text_to_draw = format!("{}{}{}", text_before_cursor, self.composition_text, text_after_cursor);
        let lines = split_lines(&text_to_draw);
        //let lines: Vec<String> = text_to_draw.lines().map(|line| line.to_string()).collect();
        for(line_idx, line) in lines.iter().enumerate(){
            let text = context.text();
            let layout = text.new_text_layout(line.to_string())
                .font(piet::FontFamily::SERIF, self.font_size)
                .text_color(color)
                .build()
                .unwrap();

            context.draw_text(&layout, 
                (self.position.x + 5.0, self.position.y + 5.0 + (line_idx as f64) * (self.font_size + self.line_gap)));
        }

        if self.selected{ self.draw_control_points(context, scale);}

        let _ = context.restore();
    }   

    fn draw_xor(&self, context: &mut WebRenderContext, state: &State){
        /*
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
        */
    }

    fn draw_control_points(&self, context: &mut WebRenderContext, scale: f64) {
        let adjusted_width = 5.0 / scale;

        let control_pts = self.control_points();
        let color = convert_to_color("#29B6F2");
        for point in control_pts{
            let rect = piet::kurbo::Rect::new(
                point.x - adjusted_width, point.y - adjusted_width, 
                point.x + adjusted_width, point.y + adjusted_width);
            context.fill(rect, &color);
        }
    }

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