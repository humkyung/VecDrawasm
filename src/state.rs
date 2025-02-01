use std::str;
use log::info;

pub struct State{
    color: String,  // 기본 색상: 파란색
    line_width: f64,// 기본 선 굵기
    scale: f64,     // 기본 스케일
}

impl State{
    pub fn new(color: String, line_width: f64) -> Self {
        State{color: color, line_width: line_width, scale: 1.0}
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
}