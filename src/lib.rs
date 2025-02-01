use log::info;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, CanvasRenderingContext2d, HtmlCanvasElement, HtmlInputElement, HtmlImageElement, MouseEvent, WheelEvent, DragEvent, FileReader, Element, Path2d
    , HtmlDivElement , DomParser, HtmlElement, Node, NodeList};
use std::rc::Rc;
use std::cell::RefCell;

pub mod shape;
use crate::shape::{Shape, Point2D, Pencil, Line, Svg};

pub mod state;
use crate::state::State;

// SHAPES 벡터 정의
thread_local! {
    static SHAPES: Rc<RefCell<Vec<Box<dyn Shape>>>> = Rc::new(RefCell::new(Vec::new()));
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_log::init_with_level(log::Level::Info).unwrap();

    // 브라우저의 Window 및 Document 객체 가져오기
    let window = web_sys::window().expect("No global window exists");
    let document = window.document().expect("Should have a document on window");

    fn request_animation_frame(window: &web_sys::Window, f: &Closure<dyn FnMut()>) {
        window
            .request_animation_frame(f.as_ref().unchecked_ref())
            .expect("should register `requestAnimationFrame` OK");
    }

    // HTML5 캔버스 가져오기
    let canvas = document
        .get_element_by_id("drawing-canvas")
        .expect("Canvas element not found")
        .dyn_into::<HtmlCanvasElement>()?;

    // 캔버스 2D 렌더링 컨텍스트 가져오기
    let context = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()?;

    let color_picker: HtmlInputElement = document
        .get_element_by_id("color-picker")
        .expect("Color picker not found")
        .dyn_into::<HtmlInputElement>()?;

    let line_width_picker = document
        .get_element_by_id("line-width")
        .expect("Line width input not found")
        .dyn_into::<HtmlInputElement>()?;

    // 오프스크린 캔버스 생성
    let offscreen_canvas = document.create_element("canvas")?.dyn_into::<HtmlCanvasElement>()?;
    offscreen_canvas.set_width(canvas.width());
    offscreen_canvas.set_height(canvas.height());
    let offscreen_context = offscreen_canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()?;

    // 초기 캔버스 상태
    let offset = Rc::new(RefCell::new((0.0, 0.0))); // 초기 X 오프셋
    let is_panning = Rc::new(RefCell::new(false));
    let is_drawing = Rc::new(RefCell::new(false)); // 드로잉 상태
    let last_mouse_pos = Rc::new(RefCell::new((0.0, 0.0)));
    let state = Rc::new(RefCell::new(State::new("#0000FF".to_string(), 2.0)));

    let animation_requested = Rc::new(RefCell::new(false));

    // 드로잉 포인트
    let mouse_context_points: Rc<RefCell<Vec<Point2D>>> = Rc::new(RefCell::new(Vec::new()));

    // 모드 설정: 팬 또는 드로잉
    let current_mode = Rc::new(RefCell::new("panning".to_string())); // "panning" 또는 "drawing"

    // 캔버스 초기화
    let fill_color = JsValue::from_str("#ffffff"); // JsValue로 변환
    offscreen_context.set_fill_style(&fill_color);
    offscreen_context.fill_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);

    // 선 추가
    SHAPES.with(|shapes| {
        let start = shape::Point2D { x: 0.0, y: 0.0 };
        let end = shape::Point2D { x: canvas.width() as f64, y: 0.0 };
        shapes.borrow_mut().push(Box::new(Line::new("#0000ff".to_string(), state.borrow().line_width(), start, end)));

        let start = shape::Point2D { x: canvas.width() as f64, y: 0.0 };
        let end = shape::Point2D { x: canvas.width() as f64, y: canvas.height() as f64 };
        shapes.borrow_mut().push(Box::new(Line::new("#0000ff".to_string(), state.borrow().line_width(), start, end)));

        let start = shape::Point2D { x: canvas.width() as f64, y: canvas.height() as f64 };
        let end = shape::Point2D { x: 0.0, y: canvas.height() as f64 };
        shapes.borrow_mut().push(Box::new(Line::new("#0000ff".to_string(), state.borrow().line_width(), start, end)));

        let start = shape::Point2D { x: 0.0, y: canvas.height() as f64 };
        let end = shape::Point2D { x: 0.0, y: 0.0 };
        shapes.borrow_mut().push(Box::new(Line::new("#0000ff".to_string(), state.borrow().line_width(), start, end)));
    });

    // 🎨 드래그 앤 드롭 이벤트 추가
    let canvas_clone = Rc::new(canvas.clone());
    let context_clone = Rc::new(context.clone());

    // ⬇️ `dragover` 이벤트: 기본 동작 방지하여 드롭 가능하게 함
    {
        let closure = Closure::wrap(Box::new(move |event: DragEvent| {
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);

        canvas.add_event_listener_with_callback("dragover", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // ⬇️ `drop` 이벤트: 파일을 읽어서 Canvas에 로드
    {
        let library_panel: HtmlDivElement = document
            .get_element_by_id("library-panel")
            .unwrap()
            .dyn_into::<HtmlDivElement>()?;

        let library_panel_clone = Rc::new(library_panel.clone());

        let context_clone = Rc::clone(&context_clone);
        let document_clone = Rc::new(document.clone());

        let closure = Closure::wrap(Box::new(move |event: DragEvent| {
            event.prevent_default();

            if let Some(data_transfer) = event.data_transfer() {
                if let Ok(svg_data) = data_transfer.get_data("text/plain") {
                    info!("svg data={svg_data}"); // 값을 콘솔에 출력

                    info!("render svg"); // 값을 콘솔에 출력
                    let rect = canvas_clone.get_bounding_client_rect();
                    let drop_x = event.client_x() as f64 - rect.left();
                    let drop_y = event.client_y() as f64 - rect.top();
                    render_svg_to_canvas(&context_clone, &canvas_clone, &svg_data, drop_x, drop_y);
                }/* else if let files = data_transfer.get_files().unwrap() {
                    if let Some(file) = files.item(0) {
                        let file_type = file.type_();

                        // ✅ SVG 파일만 허용
                        if file_type == "image/svg+xml" {
                            let reader = FileReader::new();
                            let reader_clone = reader.clone().unwrap();

                            let onload_closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
                                if let Ok(result) = reader_clone.result().as_ref().unwrap().as_string() {
                                    if let Some(svg_data) = result.as_string() {
                                        let img = HtmlImageElement::new().unwrap();
                                        let img_clone = img.clone();
                                        let context_clone = Rc::clone(&context_clone);
                                        let canvas_clone = Rc::clone(&canvas_clone);

                                        let onload_img = Closure::wrap(Box::new(move || {
                                            // 캔버스 클리어 후 이미지 그리기
                                            context_clone.clear_rect(0.0, 0.0, canvas_clone.width() as f64, canvas_clone.height() as f64);
                                            context_clone.draw_image_with_html_image_element(&img_clone, 0.0, 0.0).unwrap();
                                        }) as Box<dyn FnMut()>);

                                        // SVG 데이터를 data URL로 설정
                                        img.set_src(&format!("data:image/svg+xml,{}", svg_data));
                                        img.set_onload(Some(onload_img.as_ref().unchecked_ref()));
                                        onload_img.forget();
                                    }
                                }
                            }) as Box<dyn FnMut(_)>);

                            reader.expect("REASON").set_onload(Some(onload_closure.as_ref().unchecked_ref()));
                            reader.read_as_text(&file).unwrap();
                            onload_closure.forget();
                        }
                    }
                }*/
            }
        }) as Box<dyn FnMut(_)>);

        canvas.add_event_listener_with_callback("drop", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // 🎯 Canvas에 SVG를 벡터로 렌더링
    #[wasm_bindgen]
    pub fn render_svg_to_canvas(context: &CanvasRenderingContext2d, _canvas: &Element, svg_data: &str, x: f64, y: f64) {
        let svg = Svg::new(Point2D::new(x, y), svg_data); 
        svg.draw(context);
        SHAPES.with(|shapes| {
            shapes.borrow_mut().push(Box::new(svg));
        });
    }

    // 마우스 휠 이벤트 (줌)
    {
        let canvas_size = (canvas.width(), canvas.height());
        let offset_clone = Rc::clone(&offset);
        let context_clone = Rc::new(context.clone());
        let offscreen_context = Rc::new(offscreen_context.clone());
        let offscreen_canvas= Rc::new(offscreen_canvas.clone());
        let state_clone = Rc::clone(&state);

        let client_rect = canvas.get_bounding_client_rect();

        add_wheelevent_listener(&canvas, "wheel", move |event: WheelEvent| {
            info!("wheel"); // 값을 콘솔에 출력

            event.prevent_default();

            // 마우스 휠 방향에 따라 줌 인/아웃
            let scale= state_clone.borrow().scale();
            let zoom_factor = if event.delta_y() < 0.0 { 1.1 } else { 0.9 };
            state_clone.borrow_mut().set_scale(scale * zoom_factor);

            // 마우스 위치를 기준으로 캔버스 중심 이동
            let mouse_x = event.client_x() as f64 - client_rect.left();
            let mouse_y = event.client_y() as f64 - client_rect.top();

            let (mut offset_x, mut offset_y) = *offset_clone.borrow();
            offset_x = mouse_x - zoom_factor * (mouse_x - offset_x);
            offset_y = mouse_y - zoom_factor * (mouse_y - offset_y);
            *offset_clone.borrow_mut() = (offset_x, offset_y);

            // 잔상 방지를 위해 전체 캔버스를 리셋
            context_clone.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap(); // 변환 초기화
            context_clone.clear_rect(0.0, 0.0, client_rect.width(), client_rect.height()); // 전체 캔버스 지우기
            context_clone.set_fill_style(&"#ffffff".into());
            context_clone.fill_rect(0.0, 0.0, client_rect.width(), client_rect.height());

            // 캔버스 다시 그리기
            let _ = context_clone.set_transform( state_clone.borrow().scale(), 0.0, 0.0, state_clone.borrow().scale(), offset_x, offset_y,);
            redraw(&context_clone, &offscreen_canvas, state_clone.borrow().scale(), offset_x, offset_y);
        })?;
    }

    // 마우스 다운 이벤트 (팬 시작)
    { 
        let offset_clone = Rc::clone(&offset);

        let is_panning_clone = Rc::clone(&is_panning);
        let is_drawing = Rc::clone(&is_drawing);
        let last_mouse_pos = Rc::clone(&last_mouse_pos);
        let client_rect = canvas.get_bounding_client_rect();
        let state_clone = Rc::clone(&state);

        let mouse_context_points= Rc::clone(&mouse_context_points);

        add_event_listener(&canvas, "mousedown", move |event: MouseEvent| {
            if event.button() == 1 {
                *is_panning_clone.borrow_mut() = true;
                *is_drawing.borrow_mut() = false;
            }else {
                *is_panning_clone.borrow_mut() = false;
                *is_drawing.borrow_mut() = true;
            }

            // 마우스 위치 저장
            let mouse_x = event.client_x() as f64 - client_rect.left();
            let mouse_y = event.client_y() as f64 - client_rect.top();
            *last_mouse_pos.borrow_mut() = (mouse_x, mouse_y);

            let (offset_x, offset_y) = *offset_clone.borrow();
            let window = web_sys::window().unwrap();
            let scroll_x = window.scroll_x().unwrap_or(0.0);
            let scroll_y = window.scroll_y().unwrap_or(0.0);
            let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y), state_clone.borrow().scale(), offset_x, offset_y);
            mouse_context_points.borrow_mut().push(Point2D { x: current_x, y: current_y });

            info!("mousedown, is_panning={}, is_drawing={}", *is_panning_clone.borrow(), *is_drawing.borrow()); // 값을 콘솔에 출력
        })?;
    }

    // 마우스 이동 이벤트
    {
        let canvas= Rc::new(canvas.clone());
        let offscreen_canvas= Rc::new(offscreen_canvas.clone());

        let context_clone = Rc::new(context.clone());
        let offscreen_context = Rc::new(offscreen_context.clone());

        let canvas_size = (canvas.width(), canvas.height());
        let client_rect = canvas.get_bounding_client_rect();

        let offset_clone = Rc::clone(&offset);
        let is_panning_clone = Rc::clone(&is_panning);
        let is_drawing = Rc::clone(&is_drawing);
        let last_mouse_pos = Rc::clone(&last_mouse_pos);
        let state_clone = Rc::clone(&state);

        let mouse_context_points= Rc::clone(&mouse_context_points);

        let animation_requested_clone = Rc::clone(&animation_requested);
        let window_clone = Rc::new(window.clone());

        /*
        *g.borrow_mut() = Some(Closure::new(move || {
            if !*is_panning_clone.borrow() {
                return;
            }

            let (offset_x, offset_y) = *offset_clone.borrow();
            let _scale = *scale_clone.borrow();

            info!("animation requested scale={_scale}, offset={offset_x},{offset_y}"); // 값을 콘솔에 출력

            // 다시 그리기
            context_clone.set_transform(_scale, 0.0, 0.0, _scale, offset_x, offset_y).unwrap();
            redraw(&context_clone, &offscreen_canvas, _scale, offset_x, offset_y);

            *animation_requested_clone.borrow_mut() = false;
            request_animation_frame(&window_clone, f.borrow().as_ref().unwrap());
        }));

        let context_clone = Rc::new(context.clone());
        let is_panning_clone = Rc::clone(&is_panning);
        let offset_clone = Rc::clone(&offset);
        let scale_clone = Rc::clone(&scale);
        // 마우스 이동 이벤트에서 `requestAnimationFrame` 사용
        let _ = add_event_listener(&canvas, "mousemove", move |_event: MouseEvent| {
            let (last_x, last_y) = *last_mouse_pos.borrow();
            let (mut offset_x, mut offset_y) = *offset_clone.borrow();

            let mouse_x = _event.client_x() as f64;
            let mouse_y = _event.client_y() as f64;

            if *is_panning_clone.borrow() {
                let dx = mouse_x - last_x;
                let dy = mouse_y - last_y;

                let (mut offset_x, mut offset_y) = *offset_clone.borrow();
                offset_x += dx;
                offset_y += dy;
                *offset_clone.borrow_mut() = (offset_x, offset_y);

                *last_mouse_pos.borrow_mut() = (mouse_x, mouse_y);

                if !*animation_requested.borrow() {
                    *animation_requested.borrow_mut() = true;
                    request_animation_frame(&window, g.borrow().as_ref().unwrap());
                    info!("animation request={}", *is_panning_clone.borrow()); // 값을 콘솔에 출력
                }
            }else if *is_drawing.borrow() == true {
                let window = web_sys::window().unwrap();
                let scroll_x = window.scroll_x().unwrap_or(0.0);
                let scroll_y = window.scroll_y().unwrap_or(0.0);

                let (last_x, last_y) = calculate_canvas_coordinates((last_x, last_y), (scroll_x, scroll_y), *scale_clone.borrow(), offset_x, offset_y);
                let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y), *scale_clone.borrow(), offset_x, offset_y);

                context_clone.begin_path();
                context_clone.move_to(last_x, last_y);
                context_clone.line_to(current_x, current_y);
                context_clone.stroke();

                mouse_context_points.borrow_mut().push(Point2D { x: current_x, y: current_y });

                info!("mousemove, (last_x,last_y)=({last_x},{last_y}), (current_x, current_y)={current_x},{current_y})"); // 값을 콘솔에 출력
            }

            *last_mouse_pos.borrow_mut() = (mouse_x, mouse_y);
        })?;
        */
        add_event_listener(&canvas, "mousemove", move |event: MouseEvent| {
            let (last_x, last_y) = *last_mouse_pos.borrow();
            let (mut offset_x, mut offset_y) = *offset_clone.borrow();

            let mouse_x = event.client_x() as f64 - client_rect.left();
            let mouse_y = event.client_y() as f64 - client_rect.top();

            if *is_panning_clone.borrow() == true {
                let dx = mouse_x - last_x;
                let dy = mouse_y - last_y;

                offset_x += dx;
                offset_y += dy;
                *offset_clone.borrow_mut() = (offset_x, offset_y);

                if !*animation_requested_clone.borrow(){
                    *animation_requested_clone.borrow_mut() = true;
                    // 잔상 방지를 위해 전체 캔버스를 리셋
                    context_clone.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap(); // 변환 초기화
                    context_clone.clear_rect(0.0, 0.0, client_rect.width(), client_rect.height()); // 전체 캔버스 지우기
                    context_clone.set_fill_style(&JsValue::from_str(state_clone.borrow().color()));
                    context_clone.fill_rect(0.0, 0.0, client_rect.width(), client_rect.height());

                    // 캔버스 다시 그리기

                    let _ = context_clone.set_transform(state_clone.borrow().scale(), 0.0, 0.0, state_clone.borrow().scale(), offset_x, offset_y);
                    let draw_x = 0.0;
                    let draw_y = 0.0;
                    let _ = offscreen_context.set_transform(state_clone.borrow().scale(), 0.0, 0.0, state_clone.borrow().scale(), offset_x, offset_y);

                    redraw(&context_clone, &offscreen_canvas, state_clone.borrow().scale(), offset_x, offset_y);

                    *animation_requested_clone.borrow_mut() = false;

                    info!("panning dx={dx},dy={dy}"); // 값을 콘솔에 출력
                }
            }else if *is_drawing.borrow() == true {
                let window = web_sys::window().unwrap();
                let scroll_x = window.scroll_x().unwrap_or(0.0);
                let scroll_y = window.scroll_y().unwrap_or(0.0);

                let (last_x, last_y) = calculate_canvas_coordinates((last_x, last_y), (scroll_x, scroll_y), state_clone.borrow().scale(), offset_x, offset_y);
                let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y), state_clone.borrow().scale(), offset_x, offset_y);

                context_clone.set_stroke_style(&JsValue::from_str(state_clone.borrow().color()));
                context_clone.begin_path();
                context_clone.move_to(last_x, last_y);
                context_clone.line_to(current_x, current_y);
                context_clone.stroke();

                mouse_context_points.borrow_mut().push(Point2D { x: current_x, y: current_y });

                info!("mousemove, (last_x,last_y)=({last_x},{last_y}), (current_x, current_y)={current_x},{current_y})"); // 값을 콘솔에 출력
            }

            *last_mouse_pos.borrow_mut() = (mouse_x, mouse_y);
        })?;
    }

    // 마우스 업 이벤트 (팬 종료)
    {
        let context_clone = Rc::new(context.clone());
        let offset_clone = Rc::clone(&offset);
        let offscreen_canvas= Rc::new(offscreen_canvas.clone());
        let is_panning_clone = Rc::clone(&is_panning);
        let is_drawing = Rc::clone(&is_drawing);
        let state_clone = Rc::clone(&state);

        let mouse_context_points= Rc::clone(&mouse_context_points);

        add_event_listener(&canvas, "mouseup", move |event: MouseEvent| {
            *is_panning_clone.borrow_mut() = false;
            *is_drawing.borrow_mut() = false;

            let pencil = Pencil::new(state_clone.borrow().color().to_string(), state_clone.borrow().line_width(), mouse_context_points.borrow().clone());
            SHAPES.with(|shapes| {
                shapes.borrow_mut().push(Box::new(pencil));
            });

            mouse_context_points.borrow_mut().clear();

            let (offset_x, offset_y) = *offset_clone.borrow();

            redraw(&context_clone, &offscreen_canvas, state_clone.borrow().scale(), offset_x, offset_y);

            let num_shapes = SHAPES.with(|shapes| shapes.borrow().len());
            info!("mouseup: number of shapes={num_shapes}"); // 값을 콘솔에 출력
        })?;
    }

    // 색상 선택 이벤트
    {
        let state_clone = Rc::clone(&state);
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
    }

    // ✏️ **선 굵기 변경 이벤트 리스너 등록**
    {
        let state_clone = Rc::clone(&state);

        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            if let Some(target) = event.target() {
                if let Ok(input) = target.dyn_into::<HtmlInputElement>() {
                    if let Ok(value) = input.value().parse::<f64>() {
                        state_clone.borrow_mut().set_line_width(value);
                        info!("Line width changed to: {}", state_clone.borrow().line_width()); // 콘솔 출력
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        line_width_picker
            .add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
            .expect("Failed to add event listener");

        closure.forget(); // 메모리에서 해제되지 않도록 유지
    }

    // 지우기 버튼 이벤트
    {
        let context_clone = Rc::new(context.clone());
        let offset_clone = Rc::clone(&offset);
        let state_clone = Rc::clone(&state);

        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            SHAPES.with(|shapes| {
                shapes.borrow_mut().clear();
            });

            let (offset_x, offset_y) = *offset_clone.borrow();
            redraw(&context_clone, &offscreen_canvas, state_clone.borrow().scale(), offset_x, offset_y);
        }) as Box<dyn FnMut(_)>);

        let clear_button = document.get_element_by_id("clear-btn").unwrap();
        clear_button.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();
    }

    Ok(())
}

// 캔버스 다시 그리기
fn redraw(context: &CanvasRenderingContext2d, offscreen_canvas: &HtmlCanvasElement, scale: f64, offset_x: f64, offset_y: f64) {
    let canvas = context.canvas().unwrap();
    let canvas_width = canvas.width() as f64;
    let canvas_height = canvas.height() as f64;
    info!("redraw: canvas size=({canvas_width}, {canvas_height})"); // 값을 콘솔에 출력

    // 잔상 방지를 위해 전체 캔버스를 리셋
    context.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap(); // 변환 초기화
    context.clear_rect(0.0, 0.0, canvas_width, canvas_height); // 전체 캔버스 지우기
    context.set_fill_style(&"#ffffff".into());
    context.fill_rect(0.0, 0.0, canvas_width, canvas_height);

    // 줌 및 팬 적용 (기존의 scale과 offset 유지)
    context.set_transform(scale, 0.0, 0.0, scale, offset_x, offset_y).unwrap();

    context.clear_rect(0.0, 0.0, canvas_width, canvas_height);
    context.set_fill_style(&"#ffffff".into());
    context.fill_rect(0.0, 0.0, canvas_width, canvas_height);

    SHAPES.with(|shapes| {
        for shape in shapes.borrow().iter() {
            shape.draw(context);
        }
    });
}

/* 캔버스 좌표 계산 함수
    마우스 이벤트에서 실제 캔버스 좌표를 계산합니다.
    줌 레벨과 PAN 오프셋을 반영합니다.
*/
fn calculate_canvas_coordinates(mouse_pos: (f64, f64), scroll: (f64, f64), zoom_level: f64, offset_x: f64, offset_y: f64) -> (f64, f64) {
    let x = (mouse_pos.0 - offset_x - scroll.0) / zoom_level;
    let y = (mouse_pos.1 - offset_y - scroll.1) / zoom_level;
    return (x, y)
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