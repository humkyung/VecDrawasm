use js_sys::Promise;
use log::info;
use state::{ActionMode, DrawingMode};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::DomRect;
use web_sys::{window, Document, CanvasRenderingContext2d, HtmlCanvasElement, HtmlInputElement, HtmlImageElement, MouseEvent, WheelEvent, DragEvent, File, FileReader, Element, Path2d
    , HtmlDivElement , DomParser, HtmlElement, Node, NodeList, ImageData, Blob};
use std::rc::Rc;
use std::cell::RefCell;

pub mod shape;
use crate::shape::{Shape, Point2D, Pencil, Line, Svg};

pub mod state;
use crate::state::State;

pub mod hytos;

// SHAPES 벡터 정의
thread_local! {
    static IS_MOUSE_PRESSED: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    static STATE: Rc<RefCell<State>> = Rc::new(RefCell::new(State::new("#0000FF".to_string(), 1.0)));
    static SHAPES: Rc<RefCell<Vec<Box<dyn Shape>>>> = Rc::new(RefCell::new(Vec::new()));
    static GHOST: Rc<RefCell<Option<Box<dyn Shape>>>> = Rc::new(RefCell::new(None));
    static IMAGE_BACKUP: Rc<RefCell<Option<ImageData>>> = Rc::new(RefCell::new(None));
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

    // ✅ 모드 선택 UI
    setup_mode_buttons();

    // 초기 캔버스 상태
    let last_mouse_pos = Rc::new(RefCell::new((0.0, 0.0)));

    let animation_requested = Rc::new(RefCell::new(false));

    // 드로잉 포인트
    let mouse_context_points: Rc<RefCell<Vec<Point2D>>> = Rc::new(RefCell::new(Vec::new()));

    // 모드 설정: 팬 또는 드로잉
    let current_mode = Rc::new(RefCell::new("panning".to_string())); // "panning" 또는 "drawing"

    // 캔버스 초기화
    let fill_color = JsValue::from_str("#ffffff"); // JsValue로 변환

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
        let canvas_clone = Rc::new(canvas.clone());
        let rect = canvas_clone.get_bounding_client_rect();

        let closure = Closure::wrap(Box::new(move |event: DragEvent| {
            event.prevent_default();

            if let Some(data_transfer) = event.data_transfer() {
                if let Some(files) = data_transfer.files(){
                    for i in 0..files.length() {
                        if let Some(file) = files.get(i) {
                            let file_name = file.name();
                            if file_name.ends_with(".hytos") {
                                info!("render hytos file"); // 값을 콘솔에 출력
                                wasm_bindgen_futures::spawn_local(async move {
                                    hytos::read_sqlite_file(file).await;
                                });
                            } else if file_name.ends_with(".svg") {
                                info!("render svg file"); // 값을 콘솔에 출력
                                /*if let Ok(svg_data) = data_transfer.get_data("text/plain") {
                                    //info!("svg data={svg_data}"); // 값을 콘솔에 출력

                                    let rect = canvas_clone.get_bounding_client_rect();
                                    let mouse_x = event.client_x() as f64 - rect.left();
                                    let mouse_y = event.client_y() as f64 - rect.top();
                                    let (drop_x, drop_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (0.0, 0.0), &*state_clone.borrow());
                                    render_svg_to_canvas(&context_clone, &canvas_clone, &svg_data, drop_x, drop_y);
                                }
                                else*/{
                                    let svg_data = Rc::new(RefCell::new(String::new()));
                                    let svg_data_clone = Rc::clone(&svg_data);
                                    let context_clone = Rc::clone(&context_clone);
                                    let canvas_clone = Rc::clone(&canvas_clone);

                                    let mouse_x = event.client_x() as f64 - rect.left();
                                    let mouse_y = event.client_y() as f64 - rect.top();

                                    wasm_bindgen_futures::spawn_local(async move {
                                        let svg_content = render_svg_file(&context_clone, &canvas_clone, file, mouse_x, mouse_y).await.unwrap();
                                        *svg_data_clone.borrow_mut() = svg_content;
                                    });
                                }
                            }
                        }
                    }
                }else{
                    if let Ok(svg_data) = data_transfer.get_data("text/plain") {
                        //info!("svg data={svg_data}"); // 값을 콘솔에 출력

                        info!("render svg"); // 값을 콘솔에 출력
                        let rect = canvas_clone.get_bounding_client_rect();
                        let mouse_x = event.client_x() as f64 - rect.left();
                        let mouse_y = event.client_y() as f64 - rect.top();
                        let (drop_x, drop_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (0.0, 0.0));
                        render_svg_to_canvas(&context_clone, &canvas_clone, &svg_data, drop_x, drop_y);
                    } else {
                        let promise: Result<Promise, wasm_bindgen::JsValue> = data_transfer.get_files();
                        if let Ok(promise) = promise {
                            wasm_bindgen_futures::spawn_local(async move {
                                match JsFuture::from(promise).await {
                                    Ok(js_files) => {
                                        let files: web_sys::FileList = js_files.into();
                                        if let Some(file) = files.get(0) {
                                            hytos::read_sqlite_file(file).await;
                                        }
                                    }
                                    Err(e) => {
                                        info!("Error reading file: {:?}", e);
                                    }
                                }
                            });
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        canvas.add_event_listener_with_callback("drop", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    async fn render_svg_file(context: &CanvasRenderingContext2d, canvas: &Element, file: File, mouse_x: f64, mouse_y: f64) -> Result<String, JsValue> {
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

        let (drop_x, drop_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (0.0, 0.0));
        render_svg_to_canvas(&context, &canvas, &svg_data, drop_x, drop_y);

        Ok(svg_data)
    }

    // 🎯 Canvas에 SVG를 벡터로 렌더링
    #[wasm_bindgen]
    pub fn render_svg_to_canvas(context: &CanvasRenderingContext2d, _canvas: &Element, svg_data: &str, x: f64, y: f64) {
        info!("svg data={svg_data}"); // 값을 콘솔에 출력

        let mut svg = Svg::new(Point2D::new(x, y), svg_data); 
        svg.draw(context);
        SHAPES.with(|shapes| {
            shapes.borrow_mut().push(Box::new(svg));
        });
    }

    // 마우스 휠 이벤트 (줌)
    {
        let canvas_size = (canvas.width(), canvas.height());
        let context_clone = Rc::new(context.clone());

        let client_rect = canvas.get_bounding_client_rect();

        add_wheelevent_listener(&canvas, "wheel", move |event: WheelEvent| {
            info!("wheel"); // 값을 콘솔에 출력

            event.prevent_default();

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

                // 잔상 방지를 위해 전체 캔버스를 리셋
                context_clone.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap(); // 변환 초기화
                context_clone.clear_rect(0.0, 0.0, client_rect.width(), client_rect.height()); // 전체 캔버스 지우기
                context_clone.set_fill_style(&"#ffffff".into());
                context_clone.fill_rect(0.0, 0.0, client_rect.width(), client_rect.height());

                // 캔버스 다시 그리기
                let _ = context_clone.set_transform(state.borrow().scale(), 0.0, 0.0, state.borrow().scale(), offset.x, offset.y);
                redraw(&context_clone);
            });
        })?;
    }

    // 마우스 다운 이벤트 (팬 시작)
    { 
        let last_mouse_pos = Rc::clone(&last_mouse_pos);
        let client_rect = canvas.get_bounding_client_rect();

        let mouse_context_points= Rc::clone(&mouse_context_points);

        add_event_listener(&canvas, "mousedown", move |event: MouseEvent| {
            STATE.with(|state| {
                IS_MOUSE_PRESSED.with(|pressed| *pressed.borrow_mut() = true);

                if event.button() == 1 {
                    state.borrow_mut().set_action_mode(&state::ActionMode::Panning);
                }else {
                    state.borrow_mut().set_action_mode(&state::ActionMode::Drawing);
                }

                // ✅ 현재 캔버스 상태 백업 (이전 선택 영역 복원용)
                IMAGE_BACKUP.with(|backup| {
                    let image_data = context_clone.get_image_data(0.0, 0.0, canvas_clone.width() as f64, canvas_clone.height() as f64).unwrap();
                    *backup.borrow_mut() = Some(image_data);
                });

                // 마우스 위치 저장
                let mouse_x = event.client_x() as f64 - client_rect.left();
                let mouse_y = event.client_y() as f64 - client_rect.top();
                *last_mouse_pos.borrow_mut() = (mouse_x, mouse_y);

                let window = web_sys::window().unwrap();
                let scroll_x = window.scroll_x().unwrap_or(0.0);
                let scroll_y = window.scroll_y().unwrap_or(0.0);
                let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));
                mouse_context_points.borrow_mut().push(Point2D { x: current_x, y: current_y });
            });
        })?;
    }

    // 마우스 이동 이벤트
    {
        let canvas= Rc::new(canvas.clone());
        let context_clone = Rc::new(context.clone());

        let canvas_size = (canvas.width(), canvas.height());
        let client_rect = canvas.get_bounding_client_rect();

        let last_mouse_pos = Rc::clone(&last_mouse_pos);

        let mouse_context_points= Rc::clone(&mouse_context_points);

        let animation_requested_clone = Rc::clone(&animation_requested);
        let window_clone = Rc::new(window.clone());

        add_event_listener(&canvas, "mousemove", move |event: MouseEvent| {
            let (last_x, last_y) = *last_mouse_pos.borrow();

            let window = web_sys::window().unwrap();
            let scroll_x = window.scroll_x().unwrap_or(0.0);
            let scroll_y = window.scroll_y().unwrap_or(0.0);

            let mouse_x = event.client_x() as f64 - client_rect.left();
            let mouse_y = event.client_y() as f64 - client_rect.top();

            STATE.with(|state| {
                info!("mouse move event button={:?}", event.button()); // 값을 콘솔에 출력
                
                IS_MOUSE_PRESSED.with(|pressed|{
                    if *pressed.borrow() {
                        if state.borrow().action_mode() == &state::ActionMode::Panning {
                            let dx = mouse_x - last_x;
                            let dy = mouse_y - last_y;

                            let mut offset = state.borrow().offset().clone();
                            offset.set_x(offset.x + dx);
                            offset.set_y(offset.y + dy);
                            state.borrow_mut().set_offset(&offset);

                            if !*animation_requested_clone.borrow(){
                                *animation_requested_clone.borrow_mut() = true;
                                // 캔버스 다시 그리기
                                let scale = state.borrow().scale();
                                let _ = context_clone.set_transform(scale, 0.0, 0.0, scale, offset.x, offset.y);
                                /*IMAGE_BACKUP.with(|backup| {
                                    if let Some(ref image_data) = *backup.borrow() {
                                        context_clone.put_image_data(image_data, 0.0, 0.0).unwrap();
                                    }
                                });*/
                                redraw(&context_clone);

                                *animation_requested_clone.borrow_mut() = false;

                                info!("panning dx={dx},dy={dy}"); // 값을 콘솔에 출력
                            }
                        }else if state.borrow().action_mode() == &state::ActionMode::Drawing{
                            let (last_x, last_y) = calculate_canvas_coordinates((last_x, last_y), (scroll_x, scroll_y));
                            let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));
                            
                            let drawing_mode = *state.borrow().drawing_mode();
                            GHOST.with(|ghost|{
                                match drawing_mode {
                                    DrawingMode::Pencil =>{
                                        context_clone.set_stroke_style(&JsValue::from_str(state.borrow().color()));
                                        context_clone.begin_path();
                                        context_clone.move_to(last_x, last_y);
                                        context_clone.line_to(current_x, current_y);
                                        context_clone.stroke();

                                        mouse_context_points.borrow_mut().push(Point2D { x: current_x, y: current_y });
                                    }
                                    DrawingMode::Line =>{
                                        let start_point = *mouse_context_points.borrow().get(0).unwrap();

                                        if let Some(ref shape) = *ghost.borrow() {
                                            IMAGE_BACKUP.with(|backup| {
                                                if let Some(ref image_data) = *backup.borrow() {
                                                    context_clone.put_image_data(image_data, 0.0, 0.0).unwrap();
                                                }
                                            });
                                            //shape.draw_xor(&context_clone);
                                            //redraw(&context_clone);
                                        }

                                        let end_point = Point2D::new(current_x, current_y);
                                        let line = Line::new(state.borrow().color().to_string(), state.borrow().line_width(), start_point, end_point);
                                        line.draw_xor(&context_clone);
                                        *ghost.borrow_mut() = Some(Box::new(line));

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
                            });
                        }
                    }
                    else{
                        let (current_x, current_y) = calculate_canvas_coordinates((mouse_x, mouse_y), (scroll_x, scroll_y));

                        SHAPES.with(|shapes| {
                            let mut shapes = shapes.borrow_mut(); // 직접 mutable reference 가져오기

                            for shape in shapes.iter_mut() {
                                if shape.is_hit(current_x, current_y) {
                                    //shape.set_hovered(true);
                                } else {
                                    //shape.set_hovered(false);
                                }

                                //shape.draw(&context_clone);
                            }
                        });
                    }
                });
            });

            *last_mouse_pos.borrow_mut() = (mouse_x, mouse_y);
        })?;
    }

    // 마우스 업 이벤트 (팬 종료)
    {
        let context_clone = Rc::new(context.clone());

        let mouse_context_points= Rc::clone(&mouse_context_points);

        add_event_listener(&canvas, "mouseup", move |event: MouseEvent| {
            STATE.with(|state| {
                IS_MOUSE_PRESSED.with(|pressed| *pressed.borrow_mut() = false);
                state.borrow_mut().set_action_mode(&ActionMode::Drawing);

                // ✅ 선택 영역 확정 후, 캔버스 백업 초기화
                IMAGE_BACKUP.with(|backup| *backup.borrow_mut() = None);

                let pencil = Pencil::new(state.borrow().color().to_string(), state.borrow().line_width(), mouse_context_points.borrow().clone());
                SHAPES.with(|shapes| {
                    shapes.borrow_mut().push(Box::new(pencil));
                });

                mouse_context_points.borrow_mut().clear();

                redraw(&context_clone);

                let num_shapes = SHAPES.with(|shapes| shapes.borrow().len());
                info!("mouseup: number of shapes={num_shapes}"); // 값을 콘솔에 출력
            });
        })?;
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
                            info!("Line width changed to: {}", state_clone.borrow().line_width()); // 콘솔 출력
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
        let context_clone = Rc::new(context.clone());

        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            SHAPES.with(|shapes| {
                shapes.borrow_mut().clear();
            });

            redraw(&context_clone);
        }) as Box<dyn FnMut(_)>);

        let clear_button = document.get_element_by_id("clear-btn").unwrap();
        clear_button.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
        closure.forget();
    }

    Ok(())
}

// 캔버스 다시 그리기
fn redraw(context: &CanvasRenderingContext2d) {
    let canvas = context.canvas().unwrap();
    let canvas_width = canvas.width() as f64;
    let canvas_height = canvas.height() as f64;
    info!("redraw: canvas size=({canvas_width}, {canvas_height})"); // 값을 콘솔에 출력

    STATE.with(|state|{
        // 잔상 방지를 위해 전체 캔버스를 리셋
        context.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0).unwrap(); // 변환 초기화
        context.clear_rect(0.0, 0.0, canvas_width, canvas_height); // 전체 캔버스 지우기
        context.set_fill_style(&"#ffffff".into());
        context.fill_rect(0.0, 0.0, canvas_width, canvas_height);

        // 줌 및 팬 적용 (기존의 scale과 offset 유지)
        context.set_transform(state.borrow().scale(), 0.0, 0.0, state.borrow().scale(), state.borrow().offset().x, state.borrow().offset().y).unwrap();

        context.clear_rect(0.0, 0.0, canvas_width, canvas_height);
        context.set_fill_style(&"#ffffff".into());
        context.fill_rect(0.0, 0.0, canvas_width, canvas_height);

        SHAPES.with(|shapes| {
            for shape in shapes.borrow_mut().iter_mut() {
                shape.draw(context);
            }
        });
    });
}

fn setup_mode_buttons() {
    let document = window().unwrap().document().unwrap();

    let selection_button = document.get_element_by_id("selection-mode").unwrap();
    let pencil_button = document.get_element_by_id("pencil-mode").unwrap();
    let line_button = document.get_element_by_id("line-mode").unwrap();

    let set_mode = |mode: state::DrawingMode| {
        STATE.with(|state| state.borrow_mut().set_drawing_mode(&mode));
    };

    //add_click_listener(&selection_button, move || set_mode(state::ActionMode::Selection));
    add_click_listener(&pencil_button, move || set_mode(state::DrawingMode::Pencil));
    add_click_listener(&line_button, move || set_mode(state::DrawingMode::Line));
}

fn add_click_listener(element: &web_sys::Element, callback: impl Fn() + 'static) {
    let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
        callback();
    }) as Box<dyn FnMut(_)>);

    element.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
    closure.forget();
}

/* 캔버스 좌표 계산 함수
    마우스 이벤트에서 실제 캔버스 좌표를 계산합니다.
    줌 레벨과 PAN 오프셋을 반영합니다.
*/
fn calculate_canvas_coordinates(mouse_pos: (f64, f64), scroll: (f64, f64)) -> (f64, f64) {
    STATE.with(|state| {
        let x = (mouse_pos.0 - state.borrow().offset().x - scroll.0) / state.borrow().scale();
        let y = (mouse_pos.1 - state.borrow().offset().y - scroll.1) / state.borrow().scale();
        return (x, y);
    })
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