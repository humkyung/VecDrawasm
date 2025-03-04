use std::any::Any;
use js_sys::Math::acosh;
use js_sys::Promise;
use js_sys::Uint32Array;
use kurbo::offset;
use kurbo::Affine;
use kurbo::Point;
use log::info;
use piet_web::WebRenderContext;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::DomRect;
use web_sys::Window;
use web_sys::{window, Document, CanvasRenderingContext2d, HtmlCanvasElement, InputEvent, HtmlTextAreaElement, HtmlInputElement, HtmlImageElement, MouseEvent, WheelEvent, DragEvent, File, FileReader, Element, Path2d
    , HtmlDivElement , DomParser, HtmlElement, Node, NodeList, ImageData, Blob, KeyboardEvent, CompositionEvent, TextMetrics};
use std::char::UNICODE_VERSION;
use std::fs::OpenOptions;

use std::cell::RefCell;
use std::rc::Rc;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};
use std::thread;

use piet::{RenderContext, Color, Text, TextLayout, TextLayoutBuilder, ImageFormat, StrokeStyle, FontFamily};

use crate::shapes::geometry::BoundingRect2D;
use crate::shapes::geometry::{Point2D, Vector2D};
use std::cmp::PartialEq;
use crate::shapes::shape::{Shape, convert_to_color};
use crate::shapes::{pencil::Pencil, line::Line, rectangle::Rectangle, ellipse::Ellipse, text_box::TextBox, text_box::TextBoxManager};

use crate::state::State;

const GRID_SIZE: f64 = 50.0; // 그리드 간격

// ✅ 싱글톤 VecDrawDoc
pub struct VecDrawDoc {
    document: Document,
    canvas: HtmlCanvasElement,
    pub shapes: Vec<Arc<Mutex<Box<dyn Shape>>>>,    // ✅ 공유 가능한 Shape 리스트
    pub ghost: Option<Arc<Mutex<Box<dyn Shape>>>>,  // ✅ 공유 가능한 임시 Shape
}
unsafe impl Send for VecDrawDoc {}
unsafe impl Sync for VecDrawDoc {}

impl VecDrawDoc {
    fn new(document: Document, canvas: HtmlCanvasElement) -> Self {
        Self { 
            document,
            canvas,
            shapes: Vec::new(),
            ghost: None}
        }

    pub fn instance() -> Arc<Mutex<Self>> {
        static INSTANCE: Lazy<Arc<Mutex<VecDrawDoc>>> = Lazy::new(|| {
            let window = web_sys::window().expect("No global window exists");
            let document = window.document().expect("No document found");
            let canvas = document
                .get_element_by_id("drawing-canvas")
                .expect("Canvas element not found")
                .dyn_into::<HtmlCanvasElement>()
                .expect("Failed to cast to HtmlCanvasElement");
            Arc::new(Mutex::new(VecDrawDoc::new(document, canvas)))
        });
        Arc::clone(&INSTANCE) // ✅ 공유된 인스턴스 반환
    }

    pub fn add_shape(&mut self, shape: Box<dyn Shape>) {
        self.shapes.push(Arc::new(Mutex::new(shape)));
    }

    pub fn erase(&mut self, x: f64, y: f64, scale: f64) {
        self.shapes.retain(|shape| !shape.lock().unwrap().is_hit(x, y, scale));
    }

    pub fn clear(&mut self) {
        self.shapes.clear();
    }

    pub fn delete_selected(&mut self) {
        self.shapes.retain(|shape| !shape.lock().unwrap().is_selected());
    }

    pub fn count(&self) -> usize {
        self.shapes.len()
    }

    pub fn nth(&self, index: usize) -> Option<Arc<Mutex<Box<dyn Shape>>>> {
        self.shapes.get(index).cloned()
    }

    /// 모든 shape들을 포함하는 BoundingRect를 반환한다.
    pub fn bounding_rect(&self) -> Option<BoundingRect2D>{
        let mut res: Option<BoundingRect2D> = None; 
        self.shapes.iter().for_each(|shape|{
            let shape_rect = shape.lock().unwrap().bounding_rect();
            if res.is_some() {
                res = Some(res.unwrap() + shape_rect);
            } else {
                res = Some(shape_rect);
            }
        });

        res
    }

    /*
        마우스 커서 아래에 있는 Shape의 인덱스를 리턴한다.
    */
    pub fn get_shapes_under_mouse(&self, x: f64, y: f64, scale: f64) -> Vec<Arc<Mutex<Box<dyn Shape>>>>{
        self.shapes
            .iter()
            .filter_map(|shape| {
                if shape.lock().unwrap().is_hit(x, y, scale) {
                    Some(Arc::clone(shape))
                } else {
                    None
                }
            })
            .collect()
    }

    /*
        선택된 객체의 인덱스를 리턴한다.
    */
    pub fn get_selected_shapes(&self) -> Vec<Arc<Mutex<Box<dyn Shape>>>>{
        self.shapes
            .iter()
            .filter_map(|shape| {
                if shape.lock().unwrap().is_selected() {
                    Some(Arc::clone(shape))
                } else {
                    None
                }
            })
            .collect()
    }

    // 캔버스 다시 그리기
    pub fn draw(&self, canvas: &HtmlCanvasElement, context: &mut WebRenderContext, state: &State) {
        let canvas_width = canvas.width() as f64;
        let canvas_height = canvas.height() as f64;

        let _ = context.save();

        // 잔상 방지를 위해 전체 캔버스를 리셋
        context.transform(Affine::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])); // 변환 초기화
        let rect = piet::kurbo::Rect::new(0.0, 0.0, canvas_width, canvas_height);
        context.clear(rect, convert_to_color(state.fill_color())); // 전체 캔버스 지우기

        self.draw_ruler(context, canvas_width, canvas_height, state.world_coord());
        //self.draw_grid(&context, canvas_width, canvas_height, state.scale());

        // 줌 및 팬 적용 (기존의 scale과 offset 유지)
        let scale = state.scale();
        let offset = state.offset();
        info!("scale = {}, offset = {:?}", scale, offset);
        context.transform(Affine::new([scale, 0.0, 0.0, scale, offset.x, offset.y]));

        self.shapes.iter().for_each(|shape| {
            shape.lock().unwrap().draw(context, scale);
        });

        if let Some(ghost) = &self.ghost {
            let mut ghost = ghost.lock().unwrap();
            ghost.draw(context, state.scale());
        }

        let _ = context.restore();
    }

    /// 눈금자를 그린다.
    fn draw_ruler(&self, ctx: &mut WebRenderContext, width: f64, height: f64, word_coord: Point2D) {
        let tick_interval = 10.0;
        let major_tick_interval = 50.0;
        let text_offset = 12.0;

        let stroke_style = StrokeStyle::new();
        
        ctx.clear(None, Color::WHITE);

        // Draw horizontal ruler (top)
        for x in (0..width as i32).step_by(tick_interval as usize) {
            let height_adjust = if x as f64 % major_tick_interval == 0.0 { 10.0 } else { 5.0 };
            let line = piet::kurbo::Line::new((x as f64, 0.0), (x as f64, height_adjust));
            ctx.stroke_styled(line, &Color::BLACK, 1.0, &stroke_style);
            
            if x as f64 % major_tick_interval == 0.0 {
                let text = ctx.text();
                let layout = text.new_text_layout(format!("{}", x))
                    .default_attribute(piet::TextAttribute::FontFamily(FontFamily::SERIF))
                    .default_attribute(piet::TextAttribute::FontSize(10.0))
                    .default_attribute(piet::TextAttribute::TextColor(Color::BLACK))
                    .build()
                    .unwrap();
                ctx.draw_text(&layout, (x as f64 + 2.0, text_offset));
            }
        }
        
        // Draw vertical ruler (left)
        for y in (0..height as i32).step_by(tick_interval as usize) {
            let width_adjust = if y as f64 % major_tick_interval == 0.0 { 10.0 } else { 5.0 };
            let line = piet::kurbo::Line::new((0.0, y as f64), (width_adjust, y as f64));
            ctx.stroke_styled(line, &Color::BLACK, 1.0, &stroke_style);
            
            if y as f64 % major_tick_interval == 0.0 {
                let text = ctx.text();
                let layout = text.new_text_layout(format!("{}", y))
                    .default_attribute(piet::TextAttribute::FontFamily(FontFamily::SERIF))
                    .default_attribute(piet::TextAttribute::FontSize(10.0))
                    .default_attribute(piet::TextAttribute::TextColor(Color::BLACK))
                    .build()
                    .unwrap();
                ctx.draw_text(&layout, (text_offset, y as f64 + 2.0));
            }
        }

        // Draw mouse position indicator on rulers
        {
            let mouse_x_bar = piet::kurbo::Rect::new(word_coord.x - 1.0, 0.0, word_coord.x + 1.0, 20.0);
            let mouse_y_bar = piet::kurbo::Rect::new(0.0, word_coord.y - 1.0, 20.0, word_coord.y + 1.0);
            ctx.fill(mouse_x_bar, &Color::rgb8(255, 0, 0));
            ctx.fill(mouse_y_bar, &Color::rgb8(255, 0, 0));
        }

        // Draw mouse position indicator
        {
            let text = ctx.text();
            let layout_x = text.new_text_layout(format!("{}", word_coord.x as i32))
                .default_attribute(piet::TextAttribute::FontFamily(FontFamily::SERIF))
                .default_attribute(piet::TextAttribute::FontSize(10.0))
                .default_attribute(piet::TextAttribute::TextColor(Color::RED))
                .build()
                .unwrap();
            ctx.draw_text(&layout_x, (word_coord.x , text_offset * 2.0));
            
            let text = ctx.text();
            let layout_y = text.new_text_layout(format!("{}", word_coord.y as i32))
                .default_attribute(piet::TextAttribute::FontFamily(FontFamily::SERIF))
                .default_attribute(piet::TextAttribute::FontSize(10.0))
                .default_attribute(piet::TextAttribute::TextColor(Color::RED))
                .build()
                .unwrap();
            ctx.draw_text(&layout_y, (text_offset * 2.0, word_coord.y));
        }
    }

    // 그리드 그리기
    fn draw_grid(&self, ctx: &CanvasRenderingContext2d, width: f64, height: f64, scale: f64) {
        ctx.set_fill_style(&"#c0c0c0".into()); // 연한 회색 점

        let width = width / scale;
        let height = height / scale;
        let grid_size = GRID_SIZE / scale;

        // 점 그리드 생성
        for y in (0..height as i32).step_by(grid_size as usize) {
            for x in (0..width as i32).step_by(grid_size as usize) {
                ctx.fill_rect(x as f64, y as f64, 2.0 / scale, 2.0 / scale);
            }
        }

        // X/Y축 선 그리기
        ctx.set_stroke_style(&"#000000".into()); // 검은색
        ctx.set_line_width(1.0);

        // X축 (중앙)
        ctx.begin_path();
        ctx.move_to(0.0, height / 2.0);
        ctx.line_to(width, height / 2.0);
        ctx.stroke();

        // Y축 (중앙)
        ctx.begin_path();
        ctx.move_to(width / 2.0, 0.0);
        ctx.line_to(width / 2.0, height);
        ctx.stroke();
    }

    // shape들을 svg 문자열로 반환한다.
    pub fn to_svg(&self) -> String{
        let bound_rect = self.bounding_rect().unwrap();
        let width = bound_rect.width();
        let height = bound_rect.height();
        let mut svg_content = String::from(format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}">"#) + "\n");

        self.shapes.iter().for_each(|shape|{
            let svg = shape.lock().unwrap().to_svg(bound_rect) + "\n";
            svg_content.push_str(&svg);
        });
        svg_content.push_str("</svg>");

        svg_content
    }
}