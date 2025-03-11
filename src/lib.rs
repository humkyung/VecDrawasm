use kurbo::Point;
use piet::{RenderContext, Color, Text, TextLayout, TextLayoutBuilder, ImageFormat, StrokeStyle, FontFamily};
use kurbo::{Affine, ParamCurve, ParamCurveNearest};
use piet_web::WebRenderContext;
use shapes::shape::DrawShape;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::console::clear;
use web_sys::{window, CanvasRenderingContext2d, HtmlElement, HtmlCanvasElement, HtmlInputElement, Event, MouseEvent, WheelEvent, KeyboardEvent, CompositionEvent,
     InputEvent, Blob, BlobPropertyBag, Url, HtmlAnchorElement, DragEvent, HtmlDivElement};
use log::info;

use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};
use std::thread;

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use state::{ActionMode, DrawingMode};

mod shapes{
    pub mod geometry;
    pub mod shape;
    pub mod line;
    pub mod pencil;
    pub mod rectangle;
    pub mod polyline;
    pub mod ellipse;
    pub mod elliptical_arc;
    pub mod cubic_bez;
    pub mod text_box;
    pub mod svg;
}
mod undo_redo_manager;

use crate::shapes::geometry::{Point2D, Vector2D};
use crate::shapes::{pencil::Pencil, line::Line, rectangle::Rectangle, polyline::Polyline, ellipse::Ellipse, elliptical_arc::EllipticalArc, 
    cubic_bez::CubicBezier, text_box::TextBox, text_box::TextBoxManager};
pub mod state;
use crate::state::State;
use crate::shapes::svg::{parse_svg_file, parse_svg_data, render_svg_to_canvas};

mod vec_draw_doc;
use crate::vec_draw_doc::VecDrawDoc;

mod request;

// SHAPES 벡터 정의
thread_local! {
    static IS_MOUSE_PRESSED: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    static STATE: Rc<RefCell<State>> = Rc::new(RefCell::new(State::new("#0000FF".to_string(), 1.0)));
    static PIET_CTX: Rc<RefCell<Option<Rc<RefCell<WebRenderContext<'static>>>>>> = Rc::new(RefCell::new(None));
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_log::init_with_level(log::Level::Info).unwrap();

    // Get browser window and document
    let window = web_sys::window().ok_or("No global window exists")?;
    let document= window.document().ok_or("Should have a document on window")?;

    // Get the canvas element
    let canvas = document
        .get_element_by_id("drawing-canvas")
        .ok_or("Canvas element not found")?
        .dyn_into::<HtmlCanvasElement>()?;

    // Get rendering context
    let context = canvas
        .get_context("2d")?
        .ok_or("Failed to get 2D context")?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

    let piet_ctx = Rc::new(RefCell::new(WebRenderContext::new(context.clone(), window.clone())));
    PIET_CTX.with(|ctx|{
        *ctx.borrow_mut() = Some(piet_ctx.clone());
    });
    
    let line_width_picker = document
        .get_element_by_id("line-width")
        .expect("Line width input not found")
        .dyn_into::<HtmlInputElement>()?;

    // ✅ 모드 선택 UI
    setup_mode_buttons();
    let _ = setup_keyboard_shortcuts();

    // 초기 캔버스 상태
    let last_mouse_pos = Rc::new(RefCell::new((0.0, 0.0)));

    // show initial canvas
    {
        STATE.with(|state| {
            let instance = VecDrawDoc::instance();
            let mut doc = instance.lock().unwrap();

            /*
            let arc = EllipticalArc::new(Point2D::new(100.0, 100.0), 100.0, 50.0,
            0.0, 0.0,
            PI * 0.5, state.borrow().color().to_string(),
            state.borrow().line_width());
            doc.add_shape(Box::new(arc)); 
            let seed = js_sys::Math::random() as u64; // JS의 랜덤 함수를 이용해 시드 생성
            let mut rng = StdRng::seed_from_u64(seed); // 매번 다른 시드를 사용
            for i in 0..1000{
                let mouse_context_points_ref = mouse_context_points.borrow();
                let number1 = rng.gen_range(0..10000);
                let number2 = rng.gen_range(0..10000);
                let start = Point2D::new(number1 as f64 , number2 as f64);
                let number1 = rng.gen_range(0..10000);
                let number2 = rng.gen_range(0..10000);
                let end = Point2D::new(number1 as f64, number2 as f64);
                let line = Line::new(state.borrow().color().to_string(), state.borrow().line_width(), start, end);

                doc.add_shape(Box::new(line)); 
            }
            */
            PIET_CTX.with(|ctx|{
                if let Some(ref mut ctx) = *ctx.borrow_mut() {
                    doc.draw(&canvas, &mut ctx.borrow_mut(), &state.borrow());
                }
            });
        });
    }

    // 마우스 휠 이벤트 (줌)
    {
        let canvas_clone = Rc::new(canvas.clone());
        let context_clone = Rc::clone(&piet_ctx);

        add_wheelevent_listener(&canvas, "wheel", move |event: WheelEvent| {
            event.prevent_default();

            let client_rect = canvas_clone.get_bounding_client_rect();

            STATE.with(|state| {
                let scale = state.borrow().scale();
                let zoom_factor = if event.delta_y() < 0.0 { 1.1 } else { 0.9 };
                state.borrow_mut().set_scale(scale * zoom_factor);

                let mouse_x = event.client_x() as f64 - client_rect.left();
                let mouse_y = event.client_y() as f64 - client_rect.top();

                let mut offset = *state.borrow().offset();
                offset.set_x(mouse_x - zoom_factor * (mouse_x - offset.x));
                offset.set_y(mouse_y - zoom_factor * (mouse_y - offset.y));
                state.borrow_mut().set_offset(&offset);

                let instance = VecDrawDoc::instance();
                let doc = instance.lock().unwrap();
                doc.draw(&*canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());
            });
        })?;
    }

    // 마우스 다운 이벤트 (팬 시작)
    { 
        let last_mouse_pos = Rc::clone(&last_mouse_pos);
        let canvas_clone = Rc::new(canvas.clone());
        let context_clone = Rc::clone(&piet_ctx);
        //let mouse_context_points= Rc::clone(&mouse_context_points);

        add_event_listener(&canvas, "mousedown", move |event: MouseEvent| {
            event.prevent_default();
            let client_rect = canvas_clone.get_bounding_client_rect();

            // 마우스 위치 저장
            let mouse_x = event.client_x() as f64 - client_rect.left();
            let mouse_y = event.client_y() as f64 - client_rect.top();

            let window = web_sys::window().unwrap();
            let scroll_x = window.scroll_x().unwrap_or(0.0);
            let scroll_y = window.scroll_y().unwrap_or(0.0);

            STATE.with(|state| {
                IS_MOUSE_PRESSED.with(|pressed| *pressed.borrow_mut() = true);

                if event.button() == 1 {
                    state.borrow_mut().set_is_panning(&true);
                }else if state.borrow().action_mode() == state::ActionMode::Selection{
                    let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));

                    let instance = VecDrawDoc::instance();
                    let doc = instance.lock().unwrap();

                    let unders = doc.get_shapes_under_mouse(current_x, current_y, state.borrow().scale());
                    let selected = doc.get_selected_shapes();

                    let selection_changed: bool = (unders.is_empty() && !selected.is_empty()) || !unders.iter().all(|ele| selected.iter().any(|s| Arc::ptr_eq(ele, s)));
                    if selection_changed{
                        selected.iter().for_each(|shape| shape.lock().unwrap().set_selected(false));
                    }

                    unders.iter().for_each(|shape| {
                        let mut shape = shape.lock().unwrap();
                        let control_point_index = shape.get_control_point(current_x, current_y, state.borrow().scale());
                        shape.set_selected_control_point(control_point_index);
                        shape.set_selected(true);
                    });

                    if selection_changed{
                        doc.draw(&*canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());
                    }
                }
                else if state.borrow().action_mode() == state::ActionMode::Drawing{
                    let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));

                    let drawing_mode = state.borrow().drawing_mode();
                    match drawing_mode{
                        state::DrawingMode::Text =>{
                            let instance = TextBoxManager::instance();
                            let mut tbm= instance.lock().unwrap();

                            if !tbm.is_active(){
                                let instance = VecDrawDoc::instance();
                                let mut doc = instance.lock().unwrap();

                                doc.add_shape(Box::new(TextBox::new(current_x, current_y)));
                                // TextBoxManager 시작
                                if let Some(shape) = doc.nth(doc.count() - 1){
                                    tbm.attach(Arc::clone(&shape), &state.borrow());
                                }

                                doc.draw(&*canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());
                            }else{
                                tbm.finish_input(&state.borrow());
                            }
                        }
                        _ =>{ }
                    }

                    // 마우스 위치 저장
                    state.borrow_mut().set_world_coord(Point2D::new(current_x, current_x));
                    *last_mouse_pos.borrow_mut() = (mouse_x, mouse_y);

                    let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));
                    state.borrow_mut().mouse_points.push(Point2D { x: current_x, y: current_y });
                }
            });
        })?;
    }

    // 마우스 이동 이벤트
    {
        let canvas_clone = Rc::new(canvas.clone());
        let context_clone = Rc::clone(&piet_ctx);
        let last_mouse_pos = Rc::clone(&last_mouse_pos);

        add_event_listener(&canvas, "mousemove", move |event: MouseEvent| {
            event.prevent_default();

            let client_rect = canvas_clone.get_bounding_client_rect();

            let (last_x, last_y) = *last_mouse_pos.borrow();

            let window = web_sys::window().unwrap();
            let scroll_x = window.scroll_x().unwrap_or(0.0);
            let scroll_y = window.scroll_y().unwrap_or(0.0);

            let mouse_x = event.client_x() as f64 - client_rect.left();
            let mouse_y = event.client_y() as f64 - client_rect.top();

            STATE.with(|state| {
                canvas_clone.set_class_name("cursor-default");

                let stated = state.borrow().clone();
                let action_mode = stated.action_mode();
                let drawing_mode = stated.drawing_mode();

                let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));
                state.borrow_mut().set_world_coord(Point2D::new(current_x, current_y));
                IS_MOUSE_PRESSED.with(|pressed|{
                    if *pressed.borrow() {
                        if state.borrow().is_panning() {
                            let dx = mouse_x - last_x;
                            let dy = mouse_y - last_y;

                            let mut offset = state.borrow().offset().clone();
                            offset.set_x(offset.x + dx);
                            offset.set_y(offset.y + dy);
                            state.borrow_mut().set_offset(&offset);

                            let instance = VecDrawDoc::instance();
                            let doc = instance.lock().unwrap();
                            doc.draw(&*canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());
                        }else if action_mode == state::ActionMode::Eraser{
                            let instance = VecDrawDoc::instance();
                            let mut doc = instance.lock().unwrap();
                            doc.erase(current_x, current_y, state.borrow().scale());
                            doc.draw(&*&canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());
                        }else if action_mode == state::ActionMode::Drawing{
                            match drawing_mode {
                                DrawingMode::Pencil =>{
                                    let instance = VecDrawDoc::instance();
                                    let doc = instance.lock().unwrap();
                                    doc.draw(&*canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());

                                    state.borrow_mut().mouse_points.push(Point2D { x: current_x, y: current_y });
                                    //mouse_context_points.borrow_mut().push(Point2D { x: current_x, y: current_y });

                                    let pencil = Pencil::new(state.borrow().mouse_points.clone(),
                                    state.borrow().color().to_string(), state.borrow().line_width(),  state.borrow().background());
                                    pencil.draw_xor(&mut *context_clone.borrow_mut(), &*state.borrow());
                                }
                                _ => info!("not supported drawing mode: {drawing_mode}"), // 값을 콘솔에 출력
                            }
                        }
                        else{
                            let instance = VecDrawDoc::instance();
                            let doc = instance.lock().unwrap();

                            let selected = doc.get_selected_shapes();
                            if selected.len() > 0{
                                let (last_x, last_y) = calculate_canvas_coordinates((last_x, last_y), (scroll_x, scroll_y));
                                let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));
                                let dx = current_x - last_x;
                                let dy = current_y - last_y;

                                doc.get_selected_shapes().iter().for_each(|shape| {
                                    let mut shape = shape.lock().unwrap();
                                    let selected_control_point = shape.get_selected_control_point();
                                    if selected_control_point != -1{
                                        shape.move_control_point_by(selected_control_point, dx, dy);
                                    }else{
                                        shape.move_by(dx, dy);
                                    }
                                });

                                doc.draw(&*canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());
                            }
                        }
                    }
                    else{
                        let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));

                        let instance = VecDrawDoc::instance();
                        let doc = instance.lock().unwrap();
                        doc.draw(&*canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());

                        if action_mode == ActionMode::Drawing{
                            if state.borrow().mouse_points.len() > 0{
                                let current = Point2D::new(current_x, current_y);
                                let mouse_context_points = state.borrow().mouse_points.clone();
                                match drawing_mode{
                                    DrawingMode::Line =>{
                                        let _ = draw_xor(drawing_mode, mouse_context_points, current);
                                    }
                                    DrawingMode::Rectangle =>{
                                        let _ = draw_xor(drawing_mode, mouse_context_points, current);
                                    }
                                    DrawingMode::Polyline =>{
                                        let _ = draw_xor(drawing_mode, mouse_context_points, current);
                                    }
                                    DrawingMode::Ellipse =>{
                                        let _ = draw_xor(drawing_mode, mouse_context_points, current);
                                    }
                                    DrawingMode::EllipticalArc =>{
                                        let _ = draw_xor(drawing_mode, mouse_context_points, current);
                                    }
                                    DrawingMode::CubicBez=>{
                                        let _ = draw_xor(drawing_mode, mouse_context_points, current);
                                    }
                                    _ =>{}
                                }
                            }
                        }

                        doc.shapes.iter().for_each(|shape| {
                            if shape.lock().unwrap().is_selected(){
                                let index = shape.lock().unwrap().get_control_point(current_x, current_y, state.borrow().scale());
                                if index != -1{
                                    if index == 8{
                                        canvas_clone.set_class_name("cursor-move");
                                    }
                                    else{
                                        canvas_clone.set_class_name("cursor-pointer");//"cursor-crosshair");
                                    }
                                }
                            }else if shape.lock().unwrap().is_hit(current_x, current_y, state.borrow().scale()) {
                                shape.lock().unwrap().set_hovered(true);

                                let mut ctx = context_clone.borrow_mut(); // Context를 미리 빌려오기
                                shape.lock().unwrap().draw_xor(&mut ctx, &*state.borrow());
                            } else {
                                shape.lock().unwrap().set_hovered(false);
                            }
                        });
                    }
                });
                //state.borrow_mut().set_world_coord(Point2D::new(current_x, current_y));
            });

            *last_mouse_pos.borrow_mut() = (mouse_x, mouse_y);
        })?;
    }

    // 마우스 업 이벤트 (팬 종료)
    {
        let canvas_clone = Rc::new(canvas.clone());
        let context_clone = Rc::clone(&piet_ctx);

        add_event_listener(&canvas, "mouseup", move |event: MouseEvent| {
            event.prevent_default();

            let client_rect = canvas_clone.get_bounding_client_rect();
            let window = web_sys::window().unwrap();
            let scroll_x = window.scroll_x().unwrap_or(0.0);
            let scroll_y = window.scroll_y().unwrap_or(0.0);
            let mouse_x = event.client_x() as f64 - client_rect.left();
            let mouse_y = event.client_y() as f64 - client_rect.top();

            IS_MOUSE_PRESSED.with(|pressed| *pressed.borrow_mut() = false);
            STATE.with(|state| {
                let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));

                if state.borrow().is_panning() {
                    state.borrow_mut().set_is_panning(&false);
                    return;
                }

                let action_mode = state.borrow().action_mode();
                if action_mode == ActionMode::Drawing {
                    let drawing_mode = state.borrow().drawing_mode();
                    match drawing_mode{
                        DrawingMode::Pencil =>{
                            let mut state = state.borrow_mut();
                            let mouse_context_points = &state.mouse_points;

                            let pencil = Pencil::new(mouse_context_points.clone(),
                             state.color().to_string(), state.line_width(), state.background());

                            let instance = VecDrawDoc::instance();
                            let mut doc = instance.lock().unwrap();
                            doc.add_shape(Box::new(pencil));

                            state.mouse_points.clear();
                        }
                        DrawingMode::Polyline =>{
                            if event.button() == 2{
                                let mut state = state.borrow_mut();
                                let mouse_context_points = &state.mouse_points;

                                let polyline = Polyline::new(mouse_context_points.clone(),
                                state.color().to_string(), state.line_width(), state.background());

                                let instance = VecDrawDoc::instance();
                                let mut doc = instance.lock().unwrap();
                                doc.add_shape(Box::new(polyline));

                                state.mouse_points.clear();
                            }
                        }
                        DrawingMode::Line =>{
                            if state.borrow().mouse_points.len() == 2{
                                let mut state = state.borrow_mut();
                                let mouse_context_points = &state.mouse_points;
                                
                                let start = mouse_context_points.get(0).unwrap();
                                let end = mouse_context_points.get(1).unwrap();
                                let line = Line::new(*start, *end, state.color().to_string(), state.line_width());

                                let instance = VecDrawDoc::instance();
                                let mut doc = instance.lock().unwrap();
                                doc.add_shape(Box::new(line));

                                state.mouse_points.clear();
                            }
                        }
                        DrawingMode::Rectangle =>{
                            if state.borrow().mouse_points.len() == 2{
                                let mut state = state.borrow_mut();
                                let mouse_context_points = &state.mouse_points;

                                let start = mouse_context_points.get(0).unwrap();
                                let end = mouse_context_points.get(1).unwrap();

                                let width = end.x - start.x;
                                let height = end.y - start.y;
                                let rectangle = Rectangle::new(*start, width, height, 
                                    state.color().to_string(), state.line_width(), state.background());

                                let instance = VecDrawDoc::instance();
                                let mut doc = instance.lock().unwrap();
                                doc.add_shape(Box::new(rectangle));

                                state.mouse_points.clear();
                            }
                        }
                        DrawingMode::Ellipse =>{
                            if state.borrow().mouse_points.len() == 2{
                                let mut state = state.borrow_mut();
                                let mouse_context_points = &state.mouse_points;

                                let start = mouse_context_points.get(0).unwrap();
                                let end = mouse_context_points.get(1).unwrap();
                                let width = end.x - start.x;
                                let height = end.y - start.y;
                                let center = Point2D::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5);
                                let ellipse = Ellipse::new(center, width * 0.5, height * 0.5, 0.0, 0.0, std::f64::consts::PI * 2.0, 
                                    state.color().to_string(), state.line_width(), state.background());

                                let instance = VecDrawDoc::instance();
                                let mut doc = instance.lock().unwrap();
                                doc.add_shape(Box::new(ellipse));

                                state.mouse_points.clear();
                            }
                        }
                        DrawingMode::EllipticalArc=>{
                            if state.borrow().mouse_points.len() == 5{
                                let mut state = state.borrow_mut();
                                let mouse_context_points = &state.mouse_points;

                                let center = mouse_context_points.get(0).unwrap();
                                let p1 = mouse_context_points.get(1).unwrap();
                                let p2 = mouse_context_points.get(2).unwrap();
                                let p3 = mouse_context_points.get(3).unwrap();
                                let p4 = mouse_context_points.get(4).unwrap();

                                let dir = Vector2D::from_points(*center, *p1).normalized();
                                let dot = dir.dot(Vector2D::from_points(*center, *p2));
                                let pt = *center + dir * dot;
                                let x_axis = Vector2D::new(p1.x - center.x, p1.y - center.y);
                                let rotation = Vector2D::AXIS_X.angle_to(x_axis);

                                let arc = piet::kurbo::Arc::new(Point::new(center.x, center.y), 
                                kurbo::Vec2::new(center.distance_to(*p1), pt.distance_to(*p2)), 0.0, 2.0 * PI, rotation);
                                if let Some(closest) = closest_perimeter_point(arc, Point::new(p3.x, p3.y)){
                                    let start_axis = Vector2D::new(closest.x - center.x, closest.y - center.y);
                                    let start_angle = x_axis.angle_to(start_axis);

                                    if let Some(closest) = closest_perimeter_point(arc, Point::new(p4.x, p4.y)){
                                        let end_axis = Vector2D::new(closest.x - center.x, closest.y - center.y);

                                        let sweep_angle = start_axis.angle_to(end_axis);
                                        
                                        let rx = center.distance_to(*p1);
                                        let ry = pt.distance_to(*p2);
                                        let arc = EllipticalArc::new(*center, rx, ry, rotation, start_angle, sweep_angle, 
                                            state.color().to_string(), state.line_width(), state.background());

                                        let instance = VecDrawDoc::instance();
                                        let mut doc = instance.lock().unwrap();
                                        doc.add_shape(Box::new(arc));
                                    }
                                }

                                state.mouse_points.clear();
                            }
                        }
                        DrawingMode::CubicBez =>{
                            if state.borrow().mouse_points.len() == 4{
                                let mut state = state.borrow_mut();
                                let mouse_context_points = &state.mouse_points;

                                let p0 = mouse_context_points.get(0);
                                let p1 = mouse_context_points.get(1);
                                let p2 = mouse_context_points.get(2);
                                let p3 = mouse_context_points.get(3);

                                let bezier = CubicBezier::new(*p0.unwrap(), *p1.unwrap(), *p2.unwrap(), *p3.unwrap(), 
                                    state.color().to_string(), state.line_width(), state.background());

                                let instance = VecDrawDoc::instance();
                                let mut doc = instance.lock().unwrap();
                                doc.add_shape(Box::new(bezier));

                                state.mouse_points.clear();
                            }
                        }
                        DrawingMode::Text =>{ }
                    }

                    let instance = VecDrawDoc::instance();
                    let doc = instance.lock().unwrap();
                    doc.draw(&*canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());
                }

                state.borrow_mut().set_world_coord(Point2D::new(current_x, current_y));
            });
        })?;
    }

    // 컨텍스트 메뉴 이벤트
    {
        let document_clone = document.clone();
        let closure_contextmenu = Closure::<dyn FnMut(_)>::new(move |event: MouseEvent| {
            event.prevent_default(); // Prevent the default context menu

            let menu = document_clone.create_element("div").unwrap();
            menu.set_attribute("id", "custom-context-menu").unwrap();
            menu.set_attribute("style", "position: absolute; background: white; border: 1px solid black; padding: 5px;").unwrap();
            menu.set_inner_html("<ul style='list-style: none; margin: 0; padding: 0;'><li style='cursor: pointer;'>Option 1</li><li style='cursor: pointer;'>Option 2</li></ul>");
            
            let body = document_clone.body().unwrap();
            body.append_child(&menu).unwrap();
            
            menu.set_attribute("style", &format!("position: absolute; top: {}px; left: {}px; background: white; border: 1px solid black; padding: 5px;", event.client_y(), event.client_x())).unwrap();

            let menu_for_remove = menu.clone();
            let closure_remove_menu = Closure::<dyn FnMut(_)>::new(move |_: MouseEvent| {
                menu_for_remove.remove();
            });
            body.add_event_listener_with_callback("click", closure_remove_menu.as_ref().unchecked_ref());
            closure_remove_menu.forget();
        });
        canvas.add_event_listener_with_callback("contextmenu", closure_contextmenu.as_ref().unchecked_ref())?;
        closure_contextmenu.forget();
    }

    // ⌨️ Keyboard Input - Capture Text
    {
        let context_clone = Rc::new(context.clone());

        //let start_manager = manager.clone();
        let composition_start_closure = Closure::wrap(Box::new(move |_event: CompositionEvent| {
            let instance = TextBoxManager::instance();
            let mut tbm = instance.lock().unwrap();
            tbm.on_composition_start();
        }) as Box<dyn FnMut(_)>);

        let textarea = document.get_element_by_id("hidden-input").unwrap();
        textarea
            .add_event_listener_with_callback("compositionstart", composition_start_closure.as_ref().unchecked_ref())?;
        composition_start_closure.forget();

        // IME 조합 업데이트
        //let update_manager = manager.clone();
        let composition_update_closure = Closure::wrap(Box::new(move |event: CompositionEvent| {
            let instance = TextBoxManager::instance();
            let mut tbm = instance.lock().unwrap();
            STATE.with(|state|{
                tbm.on_composition_update(event, &state.borrow());
            });
        }) as Box<dyn FnMut(_)>);

        textarea
            .add_event_listener_with_callback("compositionupdate", composition_update_closure.as_ref().unchecked_ref())?;
        composition_update_closure.forget();

        // IME 조합 완료
        //let end_manager = manager.clone();
        let composition_end_closure = Closure::wrap(Box::new(move |event: CompositionEvent| {
            let instance = TextBoxManager::instance();
            let mut tbm = instance.lock().unwrap();
            STATE.with(|state|{
                tbm.on_composition_end(event, &state.borrow());
            });
        }) as Box<dyn FnMut(_)>);

        textarea
            .add_event_listener_with_callback("compositionend", composition_end_closure.as_ref().unchecked_ref())?;
        composition_end_closure.forget();
                          
        //let input_manager = manager.clone();
        let input_closure = Closure::wrap(Box::new(move |event: InputEvent| {
            let instance = TextBoxManager::instance();
            let mut tbm = instance.lock().unwrap();
            STATE.with(|state|{
                tbm.on_input(event, &state.borrow());
            });
        }) as Box<dyn FnMut(_)>);
        let textarea = document.get_element_by_id("hidden-input").unwrap();
        textarea.add_event_listener_with_callback("input", input_closure.as_ref().unchecked_ref())?;
        input_closure.forget();
    }

    // keyboard cursor event blinker
    { 
        let context_clone = Rc::clone(&piet_ctx);

        let window_clone = window.clone();
        // 커서 깜박임 타이머
        let closure = Closure::wrap(Box::new(move || {
            let instance = TextBoxManager::instance();
            let mut tbm = instance.lock().unwrap();
            STATE.with(|state|{
                tbm.toggle_cursor(&state.borrow());
            });
        }) as Box<dyn FnMut()>);

        window_clone.set_interval_with_callback_and_timeout_and_arguments_0(closure.as_ref().unchecked_ref(), 500)?; // 500ms마다 깜박임
        closure.forget();
    }

    // 라인 색상 선택 이벤트
    {
        info!("color picker listener");

        let color_picker: HtmlInputElement = document
        .get_element_by_id("color-picker")
        .expect("Color picker not found")
        .dyn_into::<HtmlInputElement>()?;

        STATE.with(|state|{
            let state_clone = Rc::clone(state); // ✅ `Rc<RefCell<T>>` 클론을 사용하여 상태를 전달

            let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                if let Some(target) = event.target() {
                    if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                        state_clone.borrow_mut().set_color(&input.value());
                
                        info!("Color changed to ={}", state_clone.borrow().color()); // 값을 콘솔에 출력
                    }
                }
            }) as Box<dyn FnMut(_)>);

            color_picker.add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
            .expect("Failed to add event listener");
            closure.forget();
        });
    }

    // ✏️ **선 굵기 변경 이벤트 리스너 등록**
    {
        STATE.with(|state| {
            let state_clone = Rc::clone(state); // ✅ `Rc<RefCell<T>>` 클론을 사용하여 상태를 전달

            let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
                if let Some(target) = event.target() {
                    if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                        if let Ok(value) = input.value().parse::<f64>() {
                            state_clone.borrow_mut().set_line_width(value);
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);

            line_width_picker
                .add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
                .expect("Failed to add event listener");

            closure.forget(); // 메모리에서 해제되지 않도록 유지
        });
    }

    // 배경 색상 선택 이벤트
    {
        let fill_color: HtmlInputElement = document
            .get_element_by_id("fill-color")
            .expect("fill picker not found")
            .dyn_into::<HtmlInputElement>()?;

        let fill_color_preview: HtmlElement = document
            .get_element_by_id("fillColorPreview")
            .expect("fill color preview not found")
            .dyn_into::<HtmlElement>()?;
        let fill_color_preview_clone = fill_color_preview.clone();

        let clear_fill_button = document.get_element_by_id("clear-fill").unwrap();
        let clear_fill_button_clone = clear_fill_button.clone();

        let fill_color_closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            if let Some(target) = event.target() {
                if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                    STATE.with(|state|{
                        let state_clone = Rc::clone(state); // ✅ `Rc<RefCell<T>>` 클론을 사용하여 상태를 전달
                        state_clone.borrow_mut().set_background(Some(input.value()));

                        fill_color_preview.set_class_name("");
                        fill_color_preview.style().set_property("background-color", &input.value()).unwrap();

                        clear_fill_button_clone.set_text_content(Some("Fill"));
                        clear_fill_button_clone.set_class_name("active");
                    });
                }
            }
        }) as Box<dyn FnMut(_)>);

        fill_color.add_event_listener_with_callback("input", fill_color_closure.as_ref().unchecked_ref())
        .expect("Failed to add event listener");
        fill_color_closure.forget();

        let clear_fill_button_clone = clear_fill_button.clone();

        let clear_fill_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            STATE.with(|state| {
                let state_clone = Rc::clone(state); // ✅ `Rc<RefCell<T>>` 클론을 사용하여 상태를 전달

                let caption = clear_fill_button.text_content().unwrap();
                if caption == "No Fill"{
                    let color = fill_color.value();
                    state_clone.borrow_mut().set_background(Some(color.clone()));

                    fill_color_preview_clone.set_class_name("");
                    fill_color_preview_clone.style().set_property("background-color", &color.clone()).unwrap();
                    
                    clear_fill_button.set_text_content(Some("Fill"));
                    clear_fill_button.set_class_name("active");
                }
                else{
                    state_clone.borrow_mut().set_background(None);

                    fill_color_preview_clone.set_class_name("transparent");
                    fill_color_preview_clone.style().set_property("background-color", "white").unwrap();
                    clear_fill_button.set_text_content(Some("No Fill"));
                    clear_fill_button.set_class_name("");
                }
            });
        }) as Box<dyn FnMut(_)>);

        clear_fill_button_clone.add_event_listener_with_callback("click", clear_fill_closure.as_ref().unchecked_ref())
        .expect("Failed to add clear fill event listener");
        clear_fill_closure.forget();
    }

    // Fit 버튼 이벤트
    {
        let canvas_clone = Rc::new(canvas.clone());
        let context_clone = Rc::clone(&piet_ctx);
        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            STATE.with(|state| {
                let instance = VecDrawDoc::instance();
                let mut doc = instance.lock().unwrap();
                if let Some(bounding_rect) = doc.bounding_rect(){
                    // 스케일 계산
                    let scale_x = canvas_clone.width() as f64 / bounding_rect.width();
                    let scale_y = canvas_clone.height() as f64 / bounding_rect.height();
                    let scale = scale_x.min(scale_y); // 가로/세로 중 작은 값으로 균형 맞추기
                    
                    // 중앙 정렬을 위한 오프셋 계산
                    let min = bounding_rect.min();
                    let offset_x = (canvas_clone.width() as f64 - bounding_rect.width() * scale) / 2.0 - min.x * scale;
                    let offset_y  = (canvas_clone.height() as f64 - bounding_rect.height() * scale) / 2.0 - min.y * scale;

                    state.borrow_mut().set_scale(scale);
                    state.borrow_mut().set_offset(&Point2D::new(offset_x, offset_y));

                    doc.draw(&*canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());
                }
            });
        }) as Box<dyn FnMut(_)>);

        let fit_button = document.get_element_by_id("fit-btn").unwrap();
        fit_button.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();
    }

    // 지우기 버튼 이벤트
    {
        let canvas_clone = Rc::new(canvas.clone());
        let context_clone = Rc::clone(&piet_ctx);
        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            let instance = VecDrawDoc::instance();
            let mut doc = instance.lock().unwrap();
            doc.clear();
            STATE.with(|state| {
                doc.draw(&*canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());
            });
        }) as Box<dyn FnMut(_)>);

        let clear_button = document.get_element_by_id("clear-btn").unwrap();
        clear_button.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();
    }

    // ⬇️ `dragover` 이벤트: 기본 동작 방지하여 드롭 가능하게 함
    {
        let canvas_clone = Rc::new(canvas.clone());
        let closure = Closure::wrap(Box::new(move |event: DragEvent| {
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);

        canvas_clone.add_event_listener_with_callback("dragover", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // ⬇️ `drop` 이벤트: 파일을 읽어서 Canvas에 로드
    {
        let context_clone = Rc::clone(&piet_ctx);
        let canvas_clone = Rc::new(canvas.clone());
        let canvas_clone_inner = Rc::new(canvas.clone());
        let rect = canvas_clone.get_bounding_client_rect();

        let window = web_sys::window().unwrap();
        let scroll_x = window.scroll_x().unwrap_or(0.0);
        let scroll_y = window.scroll_y().unwrap_or(0.0);

        let drop_closure = Closure::wrap(Box::new(move |event: DragEvent| {
            event.prevent_default();

            let mouse_x = event.client_x() as f64 - rect.left();
            let mouse_y = event.client_y() as f64 - rect.top();
            let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));

            if let Some(data_transfer) = event.data_transfer() {
                if let Some(files) = data_transfer.files(){
                    for i in 0..files.length() {
                        if let Some(file) = files.get(i) {
                            let file_name = file.name();
                            if file_name.ends_with(".svg") {
                                let svg_data = Rc::new(RefCell::new(String::new()));
                                let svg_data_clone = Rc::clone(&svg_data);
                                let canvas_clone_inner = Rc::clone(&canvas_clone_inner);
                                let context_clone = Rc::clone(&context_clone);

                                wasm_bindgen_futures::spawn_local(async move {
                                    let shapes = parse_svg_file(&mut *context_clone.borrow_mut(), &canvas_clone_inner, file, current_x, current_y).await.unwrap();
                                    
                                    let instance = VecDrawDoc::instance();
                                    let mut doc = instance.lock().unwrap();
                                    shapes.into_iter().for_each(|shape| {
                                        doc.add_shape(shape);
                                    });
                                });
                            }
                        }
                    }
                }else if let Ok(svg_data) = data_transfer.get_data("text/plain") {
                    let canvas_clone_inner = Rc::clone(&canvas_clone_inner);
                    render_svg_to_canvas(&mut *context_clone.borrow_mut(), &canvas_clone_inner, &svg_data, current_x, current_y);
                }
            }
        }) as Box<dyn FnMut(_)>);

        canvas_clone.add_event_listener_with_callback("drop", drop_closure.as_ref().unchecked_ref())?;
        drop_closure.forget();
    }

    Ok(())
}

const DESIRED_ACCURACY: f64 = 0.1;

/// Given a shape and a point, returns the closest position on the shape's
/// perimeter, or `None` if the shape is malformed.
fn closest_perimeter_point(shape: impl kurbo::Shape, pt: Point) -> Option<Point> {
    let mut best: Option<(Point, f64)> = None;
    for segment in shape.path_segments(DESIRED_ACCURACY) {
        let nearest = segment.nearest(pt, DESIRED_ACCURACY);
        if best.map(|(_, best_d)| nearest.distance_sq < best_d).unwrap_or(true) {
            best = Some((segment.eval(nearest.t), nearest.distance_sq))
        }
    }
    best.map(|(point, _)| point)
}

// 작성 중인 곡선을 표시한다.
fn draw_xor(drawing_mode: DrawingMode, points: Vec<Point2D>, end: Point2D) -> Result<(), JsValue>{
    if points.len()  == 0 {return Ok(());}

    // 브라우저의 Window 및 Document 객체 가져오기
    let window = web_sys::window().expect("No global window exists");
    let document = window.document().expect("Should have a document on window");

    // HTML5 캔버스 가져오기
    let canvas = document
        .get_element_by_id("drawing-canvas")
        .expect("Canvas element not found")
        .dyn_into::<HtmlCanvasElement>()?;

    // 캔버스 2D 렌더링 컨텍스트 가져오기
    // ✅ Get 2D Rendering Context
    let context = canvas
        .get_context("2d")?
        .ok_or("Failed to get 2D context")?
        .dyn_into::<CanvasRenderingContext2d>()?;

    STATE.with(|state| {
        let state = state.borrow();
        let scale = state.scale();

        // Define stroke style
        let mut stroke_style = StrokeStyle::new();
        stroke_style.set_dash_pattern([5.0 / scale, 5.0 / scale]); // Dashed line pattern
        stroke_style.set_line_cap(piet::LineCap::Round);
        stroke_style.set_line_join(piet::LineJoin::Bevel);

        let adjusted_width = 1.0 / scale;
        let color = Color::GRAY;

        PIET_CTX.with(|ctx|{
            if let Some(ref mut ctx) = *ctx.borrow_mut() {
                let mut ctx = ctx.borrow_mut();

                let _ = ctx.save();

                // 줌 및 팬 적용 (기존의 scale과 offset 유지)
                let scale = state.scale();
                let offset = state.offset();
                ctx.transform(Affine::new([scale, 0.0, 0.0, scale, offset.x, offset.y]));

                match drawing_mode{
                    DrawingMode::Line =>{
                       let line = piet::kurbo::Line::new((points.first().unwrap().x, points.first().unwrap().y), (end.x, end.y));
                       ctx.stroke_styled(line, &color, adjusted_width, &stroke_style); 
                    }
                    DrawingMode::Pencil =>{
                        let mut path = piet::kurbo::BezPath::new();
                        path.move_to(Point::new(points.first().unwrap().x, points.first().unwrap().y));

                        for point in points.iter().skip(1) {
                            path.line_to(Point::new(point.x, point.y));
                        }

                        ctx.stroke_styled(path, &color, adjusted_width, &stroke_style);
                    }
                    DrawingMode::Rectangle=>{
                        let rect = piet::kurbo::Rect::new(points.last().unwrap().x,points.last().unwrap().y, end.x, end.y);
                        ctx.stroke_styled(rect, &color, adjusted_width, &stroke_style);
                    }
                    DrawingMode::Polyline=>{
                        let mut path = piet::kurbo::BezPath::new();
                        path.move_to(Point::new(points.first().unwrap().x, points.first().unwrap().y));

                        for point in points.iter().skip(1) {
                            path.line_to(Point::new(point.x, point.y));
                        }
                        path.line_to(Point::new(end.x, end.y));

                        ctx.stroke_styled(path, &color, adjusted_width, &stroke_style);
                    }
                    DrawingMode::Ellipse=>{
                        let center = (*points.last().unwrap() + end) * 0.5;
                        let radii = piet::kurbo::Vec2::new((end.x - center.x).abs(), (end.y - center.y).abs());
                        let ellipse = piet::kurbo::Ellipse::new(Point::new(center.x, center.y), radii, 0.0);
                        ctx.stroke_styled(ellipse, &color, adjusted_width, &stroke_style);
                    }
                    DrawingMode::EllipticalArc=>{
                        if points.len() == 1{
                            let center = *points.first().unwrap();
                            let line = piet::kurbo::Line::new(Point::new(center.x, center.y), Point::new(end.x, end.y));
                            ctx.stroke_styled(line, &color, adjusted_width, &stroke_style);

                            let raddi = Vector2D::new(center.distance_to(end), center.distance_to(end));
                            let arc = piet::kurbo::Arc::new(Point::new(center.x, center.y), 
                            kurbo::Vec2::new(raddi.x, raddi.y), 0.0, 2.0*PI, 0.0);
                            ctx.stroke_styled(arc, &color, adjusted_width, &stroke_style);
                        }else if points.len() == 2{
                            let center = *points.first().unwrap();
                            let p1 = *points.get(1).unwrap();
                            let dir = Vector2D::from_points(center, p1).normalized();
                            let dot = dir.dot(Vector2D::from_points(center, end));
                            let pt = center + dir * dot;
                            let x_axis = Vector2D::new(p1.x - center.x, p1.y - center.y);
                            let rotation = Vector2D::AXIS_X.angle_to(x_axis);

                            let raddi = Vector2D::new(center.distance_to(p1), pt.distance_to(end));
                            let arc = piet::kurbo::Arc::new(Point::new(center.x, center.y), 
                            kurbo::Vec2::new(raddi.x, raddi.y), 0.0, 2.0*PI, rotation);
                            ctx.stroke_styled(arc, &color, adjusted_width, &stroke_style);

                            let line = piet::kurbo::Line::new(Point::new(center.x, center.y), Point::new(p1.x, p1.y));
                            ctx.stroke_styled(line, &color, adjusted_width, &stroke_style);

                            let mut y_axis = x_axis.clone();
                            y_axis.normalize();
                            if end.y > center.y{
                                y_axis.rotate_by(0.5 * PI);
                            }else{
                                y_axis.rotate_by(-0.5 * PI);
                            }
                            let line = piet::kurbo::Line::new(
                            Point::new(center.x, center.y),
                            Point::new(center.x + y_axis.x * raddi.y, center.y + y_axis.y * raddi.y));
                            ctx.stroke_styled(line, &color, adjusted_width, &stroke_style);
                        }else if points.len() == 3{
                            let center = *points.first().unwrap();
                            let p1 = *points.get(1).unwrap();
                            let p2 = *points.last().unwrap();
                            let dir = Vector2D::from_points(center, p1).normalized();
                            let dot = dir.dot(Vector2D::from_points(center, p2));
                            let pt = center + dir * dot;
                            let x_axis = Vector2D::new(p1.x - center.x, p1.y - center.y);
                            let rotation = Vector2D::AXIS_X.angle_to(x_axis); 

                            let raddi = Vector2D::new(center.distance_to(p1), pt.distance_to(p2));
                            let arc = piet::kurbo::Arc::new(Point::new(center.x, center.y), 
                            kurbo::Vec2::new(raddi.x, raddi.y), 0.0, 2.0 * PI, rotation);
                            ctx.stroke_styled(arc, &color, adjusted_width, &stroke_style);

                            let line = piet::kurbo::Line::new(Point::new(center.x, center.y), Point::new(p1.x, p1.y));
                            ctx.stroke_styled(line, &color, adjusted_width, &stroke_style);

                            if let Some(closest) = closest_perimeter_point(arc, Point::new(end.x, end.y)){
                                let line = piet::kurbo::Line::new(Point::new(center.x, center.y), closest);
                                ctx.stroke_styled(line, &color, adjusted_width, &stroke_style);

                                // draw mark
                                let line = piet::kurbo::Line::new(
                                    Point::new(closest.x - 5.0 / scale, closest.y - 5.0 / scale), 
                                    Point::new(closest.x + 5.0 / scale, closest.y + 5.0 / scale));
                                ctx.stroke_styled(line, &Color::BLUE, adjusted_width, &stroke_style);

                                let line = piet::kurbo::Line::new(
                                    Point::new(closest.x - 5.0 / scale, closest.y + 5.0 / scale), 
                                    Point::new(closest.x + 5.0 / scale, closest.y - 5.0 / scale));
                                ctx.stroke_styled(line, &Color::BLUE, adjusted_width, &stroke_style);
                                //

                                let start_axis = Vector2D::new(closest.x - center.x, closest.y - center.y);
                                let start_angle = x_axis.angle_to(start_axis);

                                let text = ctx.text();
                                let layout = text.new_text_layout(format!(r#"rotation = {:.1},start angle={:.1}"#, rotation * 180.0 / PI, start_angle * 180.0 / PI))
                                    .font(piet::FontFamily::SERIF, 12.0 /scale)
                                    .text_color(color)
                                    .build()
                                    .unwrap();

                                ctx.draw_text(&layout, (center.x, center.y));
                            }
                        }else if points.len() == 4{
                            let center = *points.first().unwrap();
                            let p1 = *points.get(1).unwrap();
                            let p2 = *points.get(2).unwrap();
                            let p3 = *points.last().unwrap();
                            let dir = Vector2D::from_points(center, p1).normalized();
                            let dot = dir.dot(Vector2D::from_points(center, p2));
                            let pt = center + dir * dot;
                            let x_axis = Vector2D::new(p1.x - center.x, p1.y - center.y);
                            let rotation = Vector2D::AXIS_X.angle_to(x_axis);

                            let arc = piet::kurbo::Arc::new(Point::new(center.x, center.y), 
                            kurbo::Vec2::new(center.distance_to(p1), pt.distance_to(p2)),0.0, 2.0 * PI, rotation);
                            if let Some(closest) = closest_perimeter_point(arc, Point::new(p3.x, p3.y)){
                                let start_axis = Vector2D::new(closest.x - center.x, closest.y - center.y);
                                let start_angle = x_axis.angle_to(start_axis);

                                if let Some(closest) = closest_perimeter_point(arc, Point::new(end.x, end.y)){
                                    let end_axis = Vector2D::new(closest.x - center.x, closest.y - center.y);

                                    let sweep_angle = start_axis.angle_to(end_axis);

                                    let raddi = Vector2D::new(center.distance_to(p1), pt.distance_to(p2));
                                    let arc = piet::kurbo::Arc::new(Point::new(center.x, center.y), 
                                    kurbo::Vec2::new(raddi.x, raddi.y), start_angle, sweep_angle, rotation);
                                    ctx.stroke_styled(arc, &color, adjusted_width, &stroke_style);

                                    let line = piet::kurbo::Line::new(Point::new(center.x, center.y), Point::new(p1.x, p1.y));
                                    ctx.stroke_styled(line, &color, adjusted_width, &stroke_style);

                                    let line = piet::kurbo::Line::new(Point::new(center.x, center.y), closest);
                                    ctx.stroke_styled(line, &color, adjusted_width, &stroke_style);

                                    // draw mark
                                    let line = piet::kurbo::Line::new(
                                        Point::new(closest.x - 5.0 / scale, closest.y - 5.0 / scale), 
                                        Point::new(closest.x + 5.0 / scale, closest.y + 5.0 / scale));
                                    ctx.stroke_styled(line, &Color::BLUE, adjusted_width, &stroke_style);

                                    let line = piet::kurbo::Line::new(
                                        Point::new(closest.x - 5.0 / scale, closest.y + 5.0 / scale), 
                                        Point::new(closest.x + 5.0 / scale, closest.y - 5.0 / scale));
                                    ctx.stroke_styled(line, &Color::BLUE, adjusted_width, &stroke_style);
                                    //

                                    let text = ctx.text();
                                    let layout = text.new_text_layout(format!(r#"rotation = {:.1},start angle = {:.1},sweep angle = {:.1}"#, 
                                    rotation * 180.0 / PI, start_angle * 180.0 / PI, sweep_angle * 180.0 / PI))
                                        .font(piet::FontFamily::SERIF, 12.0 /scale)
                                        .text_color(color)
                                        .build()
                                        .unwrap();

                                    ctx.draw_text(&layout, (center.x, center.y));
                                }
                            }
                        }
                    }
                    DrawingMode::CubicBez=>{
                        if points.len() == 3{
                            let p0 = points.get(0).unwrap();
                            let p1 = points.get(1).unwrap();
                            let p2 = points.get(2).unwrap();

                            let bezier = piet::kurbo::CubicBez::new(
                                kurbo::Point::new(p0.x, p0.y),
                                kurbo::Point::new(p1.x, p1.y), 
                                kurbo::Point::new(p2.x, p2.y),
                                kurbo::Point::new(end.x, end.y));
                            ctx.stroke_styled(bezier, &color, adjusted_width, &stroke_style);
                        }
                        else{
                            let mut path = piet::kurbo::BezPath::new();
                            path.move_to(Point::new(points.first().unwrap().x, points.first().unwrap().y));

                            for point in points.iter().skip(1) {
                                path.line_to(Point::new(point.x, point.y));
                            }
                            path.line_to(Point::new(end.x, end.y));

                            ctx.stroke_styled(path, &color, adjusted_width, &stroke_style);
                        }
                    }
                    _ =>{ }
                }

                let _ = ctx.restore();
            }
        });
    });

    Ok(())
}

/// Selects all shapes in `SHAPES`
fn select_all_shapes(selected: bool) -> Result<(), JsValue> {
    // 브라우저의 Window 및 Document 객체 가져오기
    let window = web_sys::window().expect("No global window exists");
    let document = window.document().expect("Should have a document on window");

    // HTML5 캔버스 가져오기
    let canvas = document
        .get_element_by_id("drawing-canvas")
        .expect("Canvas element not found")
        .dyn_into::<HtmlCanvasElement>()?;

    // 캔버스 2D 렌더링 컨텍스트 가져오기
    // ✅ Get 2D Rendering Context
    let context = canvas
        .get_context("2d")?
        .ok_or("Failed to get 2D context")?
        .dyn_into::<CanvasRenderingContext2d>()?;

    let instance = VecDrawDoc::instance();
    let mut doc = instance.lock().unwrap();
    doc.shapes.iter_mut().for_each(|shape| {
        shape.lock().unwrap().set_selected(selected);
    });

    STATE.with(|state| {
        PIET_CTX.with(|ctx|{
            if let Some(ref mut ctx) = *ctx.borrow_mut() {
                doc.draw(&canvas, &mut ctx.borrow_mut(), &*state.borrow());
            }
        });
    });

    Ok(())

}

fn setup_mode_buttons() {
    let document = window().unwrap().document().unwrap();

    let selection_button = document.get_element_by_id("selection-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
    let eraser_button = document.get_element_by_id("eraser-mode").unwrap().dyn_into::<HtmlElement>().unwrap();

    let pencil_button = document.get_element_by_id("pencil-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
    let line_button = document.get_element_by_id("line-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
    let rectangle_button = document.get_element_by_id("rectangle-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
    let polygon_button = document.get_element_by_id("polygon-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
    let ellipse_button = document.get_element_by_id("ellipse-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
    let arc_button = document.get_element_by_id("arc-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
    let bezier_button = document.get_element_by_id("bezier-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
    let text_button = document.get_element_by_id("text-mode").unwrap().dyn_into::<HtmlElement>().unwrap();

    // Function to update active button UI
    let update_ui = move |active_button: &HtmlElement| {
        let selection_button = selection_button.clone();
        let eraser_button = eraser_button.clone();
        let pencil_button = pencil_button.clone();
        let line_button = line_button.clone();
        let rectangle_button = rectangle_button.clone();
        let polygon_button = polygon_button.clone();
        let text_button = text_button.clone();

        selection_button.set_class_name("");
        eraser_button.set_class_name("");

        pencil_button.set_class_name("");
        line_button.set_class_name("");
        rectangle_button.set_class_name("");
        polygon_button.set_class_name("");
        ellipse_button.set_class_name("");
        arc_button.set_class_name("");
        bezier_button.set_class_name("");
        text_button.set_class_name("");

        active_button.set_class_name("active");
    };

    // Selection mode Handler
    {
        let selection_button = document.get_element_by_id("selection-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
        let selection_button_clone = selection_button.clone();
        let update_ui_clone = update_ui.clone();
        add_click_listener(&selection_button, move || {
            STATE.with(|state| {
                state.borrow_mut().set_action_mode(&ActionMode::Selection);
            });
            update_ui_clone(&selection_button_clone);
        });
    }

    // Eraser mode Handler
    {
        let eraser_button = document.get_element_by_id("eraser-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
        let eraser_button_clone = eraser_button.clone();
        let update_ui_clone = update_ui.clone();
        add_click_listener(&eraser_button, move || {
            STATE.with(|state| {
                state.borrow_mut().set_action_mode(&ActionMode::Eraser);
            });
            update_ui_clone(&eraser_button_clone);
        });
    }

    // Pencil mode Handler
    {
        let pencil_button = document.get_element_by_id("pencil-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
        let pencil_button_clone = pencil_button.clone();
        let update_ui_clone = update_ui.clone();
        add_click_listener(&pencil_button, move || {
            STATE.with(|state| {
                state.borrow_mut().set_action_mode(&ActionMode::Drawing);
                state.borrow_mut().set_drawing_mode(&DrawingMode::Pencil);
            });
            update_ui_clone(&pencil_button_clone);
        });
    }

    // Line mode Handler
    {
        let line_button = document.get_element_by_id("line-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
        let line_button_clone = line_button.clone();
        let update_ui_clone = update_ui.clone();
        add_click_listener(&line_button, move || {
            STATE.with(|state| {
                state.borrow_mut().set_action_mode(&ActionMode::Drawing);
                state.borrow_mut().set_drawing_mode(&DrawingMode::Line);
            });
            update_ui_clone(&line_button_clone);
        });
    }

    // Rectangle mode Handler
    {
        let rectangle_button = document.get_element_by_id("rectangle-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
        let rectangle_button_clone = rectangle_button.clone();
        let update_ui_clone = update_ui.clone();
        add_click_listener(&rectangle_button, move || {
            STATE.with(|state| {
                state.borrow_mut().set_action_mode(&ActionMode::Drawing);
                state.borrow_mut().set_drawing_mode(&DrawingMode::Rectangle);
            });
            update_ui_clone(&rectangle_button_clone);
        });
    }
    
    // Polygon mode Handler
    {
        let polygon_button = document.get_element_by_id("polygon-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
        let polygon_button_clone = polygon_button.clone();
        let update_ui_clone = update_ui.clone();
        add_click_listener(&polygon_button, move || {
            STATE.with(|state| {
                state.borrow_mut().set_action_mode(&ActionMode::Drawing);
                state.borrow_mut().set_drawing_mode(&DrawingMode::Polyline);
            });
            update_ui_clone(&polygon_button_clone);
        });
    }
    // Ellipse mode Handler
    {
        let ellipse_button = document.get_element_by_id("ellipse-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
        let ellipse_button_clone = ellipse_button.clone();
        let update_ui_clone = update_ui.clone();
        add_click_listener(&ellipse_button, move || {
            STATE.with(|state| {
                state.borrow_mut().set_action_mode(&ActionMode::Drawing);
                state.borrow_mut().set_drawing_mode(&DrawingMode::Ellipse);
            });
            update_ui_clone(&ellipse_button_clone);
        });
    }
    // Arc mode Handler
    {
        let arc_button = document.get_element_by_id("arc-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
        let arc_button_clone = arc_button.clone();
        let update_ui_clone = update_ui.clone();
        add_click_listener(&arc_button, move || {
            STATE.with(|state| {
                state.borrow_mut().set_action_mode(&ActionMode::Drawing);
                state.borrow_mut().set_drawing_mode(&DrawingMode::EllipticalArc);
            });
            update_ui_clone(&arc_button_clone);
        });
    }
    // Bezier mode Handler
    {
        let bezier_button = document.get_element_by_id("bezier-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
        let bezier_button_clone = bezier_button.clone();
        let update_ui_clone = update_ui.clone();
        add_click_listener(&bezier_button, move || {
            STATE.with(|state| {
                state.borrow_mut().set_action_mode(&ActionMode::Drawing);
                state.borrow_mut().set_drawing_mode(&DrawingMode::CubicBez);
            });
            update_ui_clone(&bezier_button_clone);
        });
    }

    // Text mode Handler
    {
        let text_button = document.get_element_by_id("text-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
        let text_button_clone = text_button.clone();
        let update_ui_clone = update_ui.clone();
        add_click_listener(&text_button, move || {
            STATE.with(|state| {
                state.borrow_mut().set_action_mode(&ActionMode::Drawing);
                state.borrow_mut().set_drawing_mode(&DrawingMode::Text);
            });
            update_ui_clone(&text_button_clone);
        });
    }
}

// keyboard event
pub fn setup_keyboard_shortcuts() -> Result<(), JsValue> {
    // 브라우저의 Window 및 Document 객체 가져오기
    let window = web_sys::window().expect("No global window exists");
    let document = window.document().expect("Should have a document on window");

    // HTML5 캔버스 가져오기
    let canvas = document
        .get_element_by_id("drawing-canvas")
        .expect("Canvas element not found")
        .dyn_into::<HtmlCanvasElement>()?;

    // 캔버스 2D 렌더링 컨텍스트 가져오기
    // ✅ Get 2D Rendering Context
    let context = canvas
        .get_context("2d")?
        .ok_or("Failed to get 2D context")?
        .dyn_into::<CanvasRenderingContext2d>()?;

    let context_clone = context.clone();
    let closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
        let instance = TextBoxManager::instance();
        let mut tbm = instance.lock().unwrap();
        if tbm.is_active(){
            STATE.with(|state| {
                let state = state.borrow();
                tbm.on_keydown(event, &state);

                let instance = VecDrawDoc::instance();
                let mut doc = instance.lock().unwrap();
                doc.delete_selected();
                PIET_CTX.with(|ctx|{
                    if let Some(ref mut ctx) = *ctx.borrow_mut() {
                        doc.draw(&canvas, &mut ctx.borrow_mut(), &state);
                    }
                });
            });
        }
        else{
            if event.ctrl_key() && event.key() == "a" {
                event.prevent_default(); // ✅ Prevent default browser "Select All" behavior
                let _ = select_all_shapes(true);
            }
            else if event.key() == "Escape"{
                event.prevent_default(); // ✅ Prevent default behavior

                let mut mouse_context_points_length = 0;
                STATE.with(|state| {
                    mouse_context_points_length = state.borrow().mouse_points.len();
                });

                if mouse_context_points_length == 0{
                    info!("mouse_context_points length == 0");
                    let _ = select_all_shapes(false);
                }else{
                    STATE.with(|state| {
                        let mut state = state.borrow_mut();
                        let mouse_context_points = &state.mouse_points;

                        info!("mouse_context_points length = {:?}", mouse_context_points.len());
                        state.mouse_points.clear();

                        let instance = VecDrawDoc::instance();
                        let mut doc = instance.lock().unwrap();
                        PIET_CTX.with(|ctx|{
                            if let Some(ref mut ctx) = *ctx.borrow_mut() {
                                doc.draw(&canvas, &mut ctx.borrow_mut(), &state);
                            }
                        });
                    });
                }
            }
            else if event.key() == "Delete"{
                event.prevent_default();

                STATE.with(|state| {
                    let instance = VecDrawDoc::instance();
                    let mut doc = instance.lock().unwrap();
                    doc.delete_selected();
                    PIET_CTX.with(|ctx|{
                        if let Some(ref mut ctx) = *ctx.borrow_mut() {
                            doc.draw(&canvas, &mut ctx.borrow_mut(), &*state.borrow());
                        }
                    });
                });
            }
        }
    }) as Box<dyn FnMut(_)>);

    window.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref()).unwrap();
    closure.forget();

    Ok(())
}

/* 캔버스 좌표 계산 함수
    마우스 이벤트에서 실제 캔버스 좌표를 계산합니다.
    줌 레벨과 PAN 오프셋을 반영합니다.
*/
fn calculate_canvas_coordinates(mouse_pos: (f64, f64), scroll: (f64, f64)) -> (f64, f64) {
    STATE.with(|state| {
        let state = state.borrow();

        let x = (mouse_pos.0 - state.offset().x) / state.scale() + scroll.0;
        let y = (mouse_pos.1 - state.offset().y) / state.scale() + scroll.1;
        return (x, y);
    })
}

fn add_click_listener(element: &web_sys::Element, callback: impl Fn() + 'static) {
    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        callback();
    }) as Box<dyn FnMut(_)>);

    element.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
    closure.forget();
}

fn add_event_listener<T>(canvas: &HtmlCanvasElement, event_type: &str, callback: T) -> Result<(), JsValue>
where
    T: 'static + FnMut(MouseEvent),
{
    let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(_)>);
    canvas.add_event_listener_with_callback(event_type, closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}

fn add_wheelevent_listener<T>(canvas: &HtmlCanvasElement, event_type: &str, callback: T) -> Result<(), JsValue>
where
    T: 'static + FnMut(WheelEvent),
{
    let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(_)>);
    canvas.add_event_listener_with_callback(event_type, closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}

#[wasm_bindgen]
pub fn svg_data_to_blob() -> Blob{

    let instance = VecDrawDoc::instance();
    let doc = instance.lock().unwrap();
    let svg_content = doc.to_svg();

    // 🔥 수정된 부분: 문자열을 `vec![...]`로 감싸서 배열 형태로 전달
    let array = js_sys::Array::new();
    array.push(&JsValue::from(svg_content));

    let mut options = BlobPropertyBag::new();
    options.type_("image/svg+xml");

    let blob = Blob::new_with_str_sequence_and_options(&array, &options)
        .expect("Failed to create Blob");
    
    blob
}

#[wasm_bindgen]
pub fn download_svg(filename: &str) {

    let instance = VecDrawDoc::instance();
    let doc = instance.lock().unwrap();
    let svg_content = doc.to_svg();

    // 🔥 수정된 부분: 문자열을 `vec![...]`로 감싸서 배열 형태로 전달
    let array = js_sys::Array::new();
    array.push(&JsValue::from(svg_content));

    let mut options = BlobPropertyBag::new();
    options.type_("image/svg+xml");

    let blob = Blob::new_with_str_sequence_and_options(&array, &options)
        .expect("Failed to create Blob");
    
    let url = Url::create_object_url_with_blob(&blob).expect("Failed to create URL");

    let document = window().unwrap().document().unwrap();
    let a = document.create_element("a").unwrap().dyn_into::<HtmlAnchorElement>().unwrap();
    a.set_href(&url);
    a.set_download(filename);
    a.style().set_property("display", "none").unwrap();

    document.body().unwrap().append_child(&a).unwrap();
    a.click();
    document.body().unwrap().remove_child(&a).unwrap();
    
    Url::revoke_object_url(&url).expect("Failed to revoke URL");
}

#[wasm_bindgen]
pub fn open_svg(svg_data: &str) {
    let window = web_sys::window().expect("No global window exists");
    let document = window.document().expect("Should have a document on window");

    // HTML5 캔버스 가져오기
    let canvas = document
        .get_element_by_id("drawing-canvas")
        .expect("Canvas element not found")
        .dyn_into::<HtmlCanvasElement>().unwrap();

    let shapes = parse_svg_data(svg_data);
    match shapes {
        Ok(shapes) =>{
            let instance = VecDrawDoc::instance();
            let mut doc = instance.lock().unwrap();
            doc.clear();
            shapes.into_iter().for_each(|shape| {
                doc.add_shape(shape);
            });

            if let Some(bounding_rect) = doc.bounding_rect(){
                // 스케일 계산
                let scale_x = canvas.width() as f64 / bounding_rect.width();
                let scale_y = canvas.height() as f64 / bounding_rect.height();
                let scale = scale_x.min(scale_y); // 가로/세로 중 작은 값으로 균형 맞추기
                
                // 중앙 정렬을 위한 오프셋 계산
                let min = bounding_rect.min();
                let offset_x = (canvas.width() as f64 - bounding_rect.width() * scale) / 2.0 - min.x * scale;
                let offset_y  = (canvas.height() as f64 - bounding_rect.height() * scale) / 2.0 - min.y * scale;

                STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    PIET_CTX.with(|ctx|{
                        if let Some(ref mut ctx) = *ctx.borrow_mut() {

                            state.set_scale(scale);
                            state.set_offset(&Point2D::new(offset_x, offset_y));

                            doc.draw(&canvas, &mut ctx.borrow_mut(), &state);
                        }
                    });
                });
            }
        }
        Err(error) =>{
            info!("There is a problem opening svg data: {:?}", error);
        }
    };
}