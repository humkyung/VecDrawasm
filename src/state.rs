use std::{mem::offset_of, str, fmt};
use log::info;

use crate::shape::Point2D;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ActionMode {
    Selection,
    Panning,
    Drawing,
}

impl fmt::Display for ActionMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ActionMode::Selection => write!(f, "Selection Mode"),
            ActionMode::Panning => write!(f, "Panning Mode"),
            ActionMode::Drawing => write!(f, "Drawing Mode"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum DrawingMode {
    Pencil,
    Line,
}

impl fmt::Display for DrawingMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DrawingMode::Pencil => write!(f, "Pencil Mode"),
            DrawingMode::Line => write!(f, "Line Mode"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct State{
    action_mode: ActionMode,
    drawing_mode: DrawingMode,
    color: String,  // 기본 색상: 파란색
    line_width: f64,// 기본 선 굵기
    scale: f64,     // 기본 스케일
    offset: Point2D
}

impl State{
    pub fn new(color: String, line_width: f64) -> Self {
        State{action_mode: ActionMode::Drawing, drawing_mode: DrawingMode::Line, color: color, line_width: line_width, scale: 1.0, offset: Point2D::new(0.0, 0.0)}
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

    pub fn action_mode(&self) -> &ActionMode {
        &self.action_mode
    }

    pub fn set_action_mode(&mut self, value: &ActionMode) {
        self.action_mode = value.clone();
        info!("selected mode {:?}", self.action_mode);
    }

    pub fn drawing_mode(&self) -> &DrawingMode{
        &self.drawing_mode
    }

    pub fn set_drawing_mode(&mut self, value: &DrawingMode) {
        self.drawing_mode = value.clone();
    }
}