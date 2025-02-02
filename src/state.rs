use std::{mem::offset_of, str};
use log::info;

use crate::shape::Point2D;

pub struct State{
    color: String,  // 기본 색상: 파란색
    line_width: f64,// 기본 선 굵기
    scale: f64,     // 기본 스케일
    offset: Point2D
}

impl State{
    pub fn new(color: String, line_width: f64) -> Self {
        State{color: color, line_width: line_width, scale: 1.0, offset: Point2D::new(0.0, 0.0)}
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
}