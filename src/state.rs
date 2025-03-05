use std::{mem::offset_of, str, fmt};
use kurbo::Point;
use log::info;

use crate::shapes::geometry::{Point2D};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ActionMode {
    Selection,
    Eraser,
    Drawing,
}

impl fmt::Display for ActionMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ActionMode::Selection => write!(f, "Selection Mode"),
            ActionMode::Eraser => write!(f, "Eraser Mode"),
            ActionMode::Drawing => write!(f, "Drawing Mode"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DrawingMode {
    Pencil,
    Line,
    Rectangle,
    Polyline,
    Ellipse,
    CubicBez,
    Text,
}

impl fmt::Display for DrawingMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DrawingMode::Pencil => write!(f, "Pencil Mode"),
            DrawingMode::Line => write!(f, "Line Mode"),
            DrawingMode::Rectangle => write!(f, "Rectangle Mode"),
            DrawingMode::Polyline => write!(f, "Polyline Mode"),
            DrawingMode::Ellipse => write!(f, "Ellipse Mode"),
            DrawingMode::CubicBez => write!(f, "CubicBez Mode"),
            DrawingMode::Text => write!(f, "Text Mode"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct State{
    is_panning: bool,
    action_mode: ActionMode,
    drawing_mode: DrawingMode,
    world_coord: Point2D,
    color: String,  // 기본 색상: 파란색
    background: Option<String>, // 배경색
    line_width: f64,// 기본 선 굵기
    scale: f64,     // 기본 스케일
    offset: Point2D,
    fill_color: String,
    selected_control_point: Option<(i32, i32)>  // shape index, control point index
}

impl State{
    pub fn new(color: String, line_width: f64) -> Self {
        State {
            is_panning: false,
            action_mode: ActionMode::Drawing,
            drawing_mode: DrawingMode::CubicBez,
            world_coord: Point2D::new(0.0,  0.0),
            color: color,
            background: None,
            line_width: line_width,
            scale: 1.0,
            offset: Point2D::new(0.0, 0.0),
            fill_color: String::from("#ffffff"),
            selected_control_point: None
        }
    }

    pub fn fill_color(&self) -> &str{
        &self.fill_color
    }

    pub fn color(&self) -> &str {
        &self.color
    }

    pub fn set_color(&mut self, value: &String) {
        self.color = value.clone();
    }

    pub fn line_width(&self) -> f64 {
        self.line_width
    }

    pub fn set_line_width(&mut self, value: f64) {
        self.line_width = value;
    }

    pub fn background(&self) -> Option<String>{
        self.background.clone()
    }

    pub fn set_background(&mut self, value: Option<String>){
        self.background = value;
    }

    pub fn world_coord(&self) -> Point2D{
        self.world_coord
    }

    pub fn set_world_coord(&mut self, value: Point2D){
        self.world_coord = value;
    }

    pub fn scale(&self) -> f64 {
        self.scale
    }

    pub fn set_scale(&mut self, value: f64) {
        self.scale = value;
    }
    
    pub fn offset(&self) -> &Point2D{
        &self.offset
    }

    pub fn offset_mut(&mut self) -> &mut Point2D{
        &mut self.offset
    }

    pub fn set_offset(&mut self, value: &Point2D){
        self.offset.set_x(value.x);
        self.offset.set_y(value.y);
    }

    pub fn is_panning(&self) -> bool{
        self.is_panning
    }

    pub fn set_is_panning(&mut self, value: &bool){
        self.is_panning = value.clone();
    }

    pub fn action_mode(&self) -> ActionMode {
        self.action_mode
    }

    pub fn set_action_mode(&mut self, value: &ActionMode) {
        self.action_mode = value.clone();
    }

    pub fn drawing_mode(&self) -> DrawingMode{
        self.drawing_mode
    }

    pub fn set_drawing_mode(&mut self, value: &DrawingMode) {
        self.drawing_mode = value.clone();
    }

    pub fn selected_control_point(&self) -> Option<(i32, i32)>{
        self.selected_control_point
    }

    pub fn set_selected_control_point(&mut self, value: Option<(i32, i32)>){
        self.selected_control_point = value;
    }
}