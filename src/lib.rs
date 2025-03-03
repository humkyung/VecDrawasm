use kurbo::Point;
use piet::{RenderContext, Color, Text, TextLayout, TextLayoutBuilder, ImageFormat, StrokeStyle, FontFamily};
use kurbo::{Affine};
use piet_web::WebRenderContext;
use shapes::shape::Shape;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, CanvasRenderingContext2d, HtmlElement, HtmlCanvasElement, HtmlInputElement, MouseEvent, WheelEvent, KeyboardEvent, CompositionEvent, InputEvent};
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
    pub mod ellipse;
    pub mod elliptical_arc;
    pub mod text_box;
}
use crate::shapes::geometry::{Point2D, Vector2D};
use crate::shapes::{pencil::Pencil, line::Line, rectangle::Rectangle, ellipse::Ellipse, elliptical_arc::EllipticalArc,
     text_box::TextBox, text_box::TextBoxManager};

pub mod state;
use crate::state::State;

mod vec_draw_doc;
use crate::vec_draw_doc::VecDrawDoc;

mod generate_pdf; // 📌 `piet_svg.rs` 모듈 불러오기

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
    
    let color_picker: HtmlInputElement = document
        .get_element_by_id("color-picker")
        .expect("Color picker not found")
        .dyn_into::<HtmlInputElement>()?;

    let line_width_picker = document
        .get_element_by_id("line-width")
        .expect("Line width input not found")
        .dyn_into::<HtmlInputElement>()?;

    // ✅ 모드 선택 UI
    setup_mode_buttons();
    let _ = setup_keyboard_shortcuts();

    // 초기 캔버스 상태
    let last_mouse_pos = Rc::new(RefCell::new((0.0, 0.0)));
    // 드로잉 포인트
    let mouse_context_points: Rc<RefCell<Vec<Point2D>>> = Rc::new(RefCell::new(Vec::new()));

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
        let mouse_context_points= Rc::clone(&mouse_context_points);

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
                }else if state.borrow().action_mode() == &state::ActionMode::Selection{
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
                else if state.borrow().action_mode() == &state::ActionMode::Drawing{
                    let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));
                    if state.borrow().drawing_mode() == &state::DrawingMode::Text{
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

                    // 마우스 위치 저장
                    state.borrow_mut().set_world_coord(Point::new(current_x, current_x));
                    *last_mouse_pos.borrow_mut() = (mouse_x, mouse_y);

                    let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));
                    mouse_context_points.borrow_mut().push(Point2D { x: current_x, y: current_y });
                }
            });
        })?;
    }

    // 마우스 이동 이벤트
    {
        let canvas_clone = Rc::new(canvas.clone());
        let context_clone = Rc::clone(&piet_ctx);
        let last_mouse_pos = Rc::clone(&last_mouse_pos);
        let mouse_context_points= Rc::clone(&mouse_context_points);

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

                let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));
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
                        }else if state.borrow().action_mode() == &state::ActionMode::Eraser{
                            let instance = VecDrawDoc::instance();
                            let mut doc = instance.lock().unwrap();
                            doc.erase(current_x, current_y, state.borrow().scale());
                            doc.draw(&*&canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());
                        }else if state.borrow().action_mode() == &state::ActionMode::Drawing{
                            let (last_x, last_y) = calculate_canvas_coordinates((last_x, last_y), (scroll_x, scroll_y));
                            
                            let instance = VecDrawDoc::instance();
                            let doc = instance.lock().unwrap();
                            doc.draw(&*canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());

                            let drawing_mode = *state.borrow().drawing_mode();
                            match drawing_mode {
                                DrawingMode::Pencil =>{
                                    mouse_context_points.borrow_mut().push(Point2D { x: current_x, y: current_y });

                                    let pencil = Pencil::new(state.borrow().color().to_string(), state.borrow().line_width(), mouse_context_points.borrow().clone());
                                    pencil.draw_xor(&mut *context_clone.borrow_mut(), &*state.borrow());
                                }
                                DrawingMode::Line =>{
                                    let start_point = *mouse_context_points.borrow().get(0).unwrap();
                                    let end_point = Point2D::new(current_x, current_y);
                                    let mut ctx = context_clone.borrow_mut(); // Context를 미리 빌려오기

                                    let line = Line::new(state.borrow().color().to_string(), state.borrow().line_width(), start_point, end_point);
                                    line.draw_xor(&mut *ctx, &*state.borrow());

                                    if mouse_context_points.borrow().len() == 1{
                                        mouse_context_points.borrow_mut().push(end_point);
                                    }
                                    else{
                                        mouse_context_points.borrow_mut().remove(1);
                                        mouse_context_points.borrow_mut().push(end_point);
                                    }
                                }
                                DrawingMode::Rectangle =>{
                                    let start_point = *mouse_context_points.borrow().get(0).unwrap();

                                    let end_point = Point2D::new(current_x, current_y);
                                    let width = end_point.x - start_point.x;
                                    let height = end_point.y - start_point.y;
                                    let rectangle = Rectangle::new(state.borrow().color().to_string(), state.borrow().line_width(), start_point, width, height);

                                    let mut ctx = context_clone.borrow_mut(); // Context를 미리 빌려오기
                                    rectangle.draw_xor(&mut *ctx, &*state.borrow());

                                    if mouse_context_points.borrow().len() == 1{
                                        mouse_context_points.borrow_mut().push(end_point);
                                    }
                                    else{
                                        mouse_context_points.borrow_mut().remove(1);
                                        mouse_context_points.borrow_mut().push(end_point);
                                    }
                                }
                                DrawingMode::Ellipse =>{
                                    let start_point = *mouse_context_points.borrow().get(0).unwrap();

                                    let end_point = Point2D::new(current_x, current_y);
                                    let width = end_point.x - start_point.x;
                                    let height = end_point.y - start_point.y;
                                    let center = Point2D::new(current_x - width * 0.5, current_y - height * 0.5);
                                    let ellipse= Ellipse::new(center, width * 0.5, height * 0.5, 0.0, 0.0, std::f64::consts::PI * 2.0, state.borrow().color().to_string(), state.borrow().line_width());

                                    let mut ctx = context_clone.borrow_mut(); // Context를 미리 빌려오기
                                    ellipse.draw_xor(&mut *ctx, &*state.borrow());

                                    if mouse_context_points.borrow().len() == 1{
                                        mouse_context_points.borrow_mut().push(end_point);
                                    }
                                    else{
                                        mouse_context_points.borrow_mut().remove(1);
                                        mouse_context_points.borrow_mut().push(end_point);
                                    }
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
                            } else {
                                shape.lock().unwrap().set_hovered(false);
                            }

                            let mut ctx = context_clone.borrow_mut(); // Context를 미리 빌려오기
                            shape.lock().unwrap().draw_xor(&mut ctx, &*state.borrow());
                        });
                    }
                });
                state.borrow_mut().set_world_coord(Point::new(current_x, current_y));
            });

            *last_mouse_pos.borrow_mut() = (mouse_x, mouse_y);
        })?;
    }

    // 마우스 업 이벤트 (팬 종료)
    {
        let canvas_clone = Rc::new(canvas.clone());
        let context_clone = Rc::clone(&piet_ctx);
        let mouse_context_points= Rc::clone(&mouse_context_points);

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

                if *state.borrow().action_mode() == ActionMode::Drawing {
                    let state_ref = state.borrow();
                    let drawing_mode = state_ref.drawing_mode();
                    match drawing_mode{
                        DrawingMode::Pencil =>{
                            let pencil = Pencil::new(state.borrow().color().to_string(), state.borrow().line_width(), mouse_context_points.borrow().clone());

                            let instance = VecDrawDoc::instance();
                            let mut doc = instance.lock().unwrap();
                            doc.add_shape(Box::new(pencil));
                        }
                        DrawingMode::Line =>{
                            let mouse_context_points_ref = mouse_context_points.borrow();
                            let start = mouse_context_points_ref.get(0).unwrap();
                            let end = mouse_context_points_ref.get(mouse_context_points.borrow().len() - 1).unwrap();
                            let line = Line::new(state.borrow().color().to_string(), state.borrow().line_width(), *start, *end);

                            let instance = VecDrawDoc::instance();
                            let mut doc = instance.lock().unwrap();
                            doc.add_shape(Box::new(line));
                        }
                        DrawingMode::Rectangle =>{
                            let mouse_context_points_ref = mouse_context_points.borrow();
                            let start = mouse_context_points_ref.get(0).unwrap();
                            let end = mouse_context_points_ref.get(mouse_context_points.borrow().len() - 1).unwrap();
                            let width = end.x - start.x;
                            let height = end.y - start.y;
                            let rectangle = Rectangle::new(state.borrow().color().to_string(), state.borrow().line_width(), *start, width, height);

                            let instance = VecDrawDoc::instance();
                            let mut doc = instance.lock().unwrap();
                            doc.add_shape(Box::new(rectangle));
                        }
                        DrawingMode::Ellipse =>{
                            let mouse_context_points_ref = mouse_context_points.borrow();
                            let start = mouse_context_points_ref.get(0).unwrap();
                            let end = mouse_context_points_ref.get(mouse_context_points.borrow().len() - 1).unwrap();
                            let width = end.x - start.x;
                            let height = end.y - start.y;
                            let center = Point2D::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5);
                            let ellipse = Ellipse::new(center, width * 0.5, height * 0.5, 0.0, 0.0, std::f64::consts::PI * 2.0, state.borrow().color().to_string(), state.borrow().line_width());

                            let instance = VecDrawDoc::instance();
                            let mut doc = instance.lock().unwrap();
                            doc.add_shape(Box::new(ellipse));
                        }
                        DrawingMode::Text =>{ }
                    }

                    let instance = VecDrawDoc::instance();
                    let doc = instance.lock().unwrap();
                    doc.draw(&*canvas_clone, &mut context_clone.borrow_mut(), &*state.borrow());
                }

                mouse_context_points.borrow_mut().clear();
                state.borrow_mut().set_world_coord(Point::new(current_x, current_y));
            });
        })?;
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

    // 색상 선택 이벤트
    {
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
    let ellipse_button = document.get_element_by_id("ellipse-mode").unwrap().dyn_into::<HtmlElement>().unwrap();
    let text_button = document.get_element_by_id("text-mode").unwrap().dyn_into::<HtmlElement>().unwrap();

    // Function to update active button UI
    let update_ui = move |active_button: &HtmlElement| {
        let selection_button = selection_button.clone();
        let eraser_button = eraser_button.clone();
        let pencil_button = pencil_button.clone();
        let line_button = line_button.clone();
        let rectangle_button = rectangle_button.clone();
        let text_button = text_button.clone();

        selection_button.set_class_name("");
        eraser_button.set_class_name("");

        pencil_button.set_class_name("");
        line_button.set_class_name("");
        rectangle_button.set_class_name("");
        ellipse_button.set_class_name("");
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
                let _ = select_all_shapes(false);
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