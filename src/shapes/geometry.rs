use std::ops::{self, Add, AddAssign, Sub, SubAssign, Mul};

use kurbo::Point;

#[derive(Debug, Clone, Copy)]
pub struct Point2D{
    pub x: f64,
    pub y: f64,
}
impl Point2D{
    pub fn new(x: f64, y: f64) -> Self {
        Point2D{x, y}
    }

    pub fn set_x(&mut self, value: f64){
        self.x = value;
    } 

    pub fn set_y(&mut self, value: f64){
        self.y = value;
    }

    pub fn to_string(&self) -> String{
        format!(r#"{:.3},{:.3}"#, self.x, self.y)
    }
}
impl Add<Point2D> for Point2D{
    type Output = Self;

    fn add(self, other: Point2D) -> Self{
        Self{x: self.x + other.x, y: self.y + other.y}
    }
}
impl AddAssign<Point2D> for Point2D{
    fn add_assign(&mut self, rhs: Point2D) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
impl AddAssign<Vector2D> for Point2D{
    fn add_assign(&mut self, rhs: Vector2D) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
impl Sub for Point2D {
    type Output = Point2D;

    fn sub(self, other: Point2D) -> Point2D {
        Point2D {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}
impl SubAssign for Point2D {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;    
    }
}

impl Mul<f64> for Point2D{
    type Output = Self;

    fn mul(self, other: f64) -> Self{
        Self{x: self.x * other, y: self.y * other}
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Vector2D{
    pub x: f64,
    pub y: f64,
}
impl Vector2D{
    pub const AXIS_X: Vector2D = Vector2D { x: 1.0, y: 0.0 };
    pub const AXIS_Y: Vector2D = Vector2D { x: 0.0, y: 1.0 };

    pub fn new(x: f64, y: f64) -> Self {
        Vector2D{x, y}
    }

    pub fn from_points(start: Point2D, end: Point2D) -> Self{
        Vector2D{x: end.x - start.x, y: end.y - start.y}
    }

    pub fn set_x(&mut self, value: f64){
        self.x = value;
    } 

    pub fn set_y(&mut self, value: f64){
        self.y = value;
    }

    pub fn length(&self) -> f64{
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalize(&mut self){
        let length = self.length();
        self.x /= length;
        self.y /= length;
    }

    pub fn dot(&self, vec: Vector2D) -> f64{
        self.x * vec.x + self.y * vec.y
    }

    pub fn cross(&self, vec: Vector2D) -> f64{
        self.x * vec.y - self.y * vec.x
    }

    /*
        return the angle in radian between two vectors
    */
    pub fn angle_to(&self, vec: Vector2D) -> f64{
        let mut radian = (self.dot(vec) / (self.length() * vec.length())).acos();
        let norm = self.cross(vec);
        if norm < 0.0 {radian = -radian;}

        radian
    }

    pub fn rotate_by(&mut self, rotation: f64){
        let cos = rotation.cos();
        let sin = rotation.sin();
        let x = cos * self.x - sin * self.y;
        let y = sin * self.x + cos * self.y;
        self.x = x;
        self.y = y;
    }
}

impl Mul<f64> for Vector2D{
    type Output = Self;

    fn mul(self, other: f64) -> Self{
        Self{x: self.x * other, y: self.y * other}
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BoundingRect2D{
    pub min: Point2D,
    pub max: Point2D
}
impl BoundingRect2D{
    pub fn new(min: Point2D, max: Point2D) -> Self {
        BoundingRect2D{min, max}
    }

    // Point Vector에서 BoundingRect를 반환한다.
    pub fn from_points(points: Vec<Point2D>) -> Self{
        let mut min = points[0];
        let mut max = points[0];
        points.iter().skip(1).for_each(|point| {
            if min.x > point.x { min.x = point.x;}
            if min.y > point.y { min.y = point.y;}
            if max.x < point.x { max.x = point.x;}
            if max.y < point.y { max.y = point.y;}
        });

        BoundingRect2D{min, max}
    }

    pub fn min(&self) -> Point2D{
        self.min
    }

    pub  fn max(&self) -> Point2D{
        self.max
    }

    // BoundingRect의 중점을 리턴한다.
    pub fn center(&self) -> Point2D{
        (self.min + self.max) * 0.5
    }

    pub fn width(&self) -> f64{
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f64{
        self.max.y - self.min.y
    }
}

impl Add<BoundingRect2D> for BoundingRect2D{
    type Output = Self;

    fn add(self, other: BoundingRect2D) -> Self{
        Self{
            min: Point2D{x: self.min.x.min(other.min.x), y: self.min.y.min(other.min.y)},
            max: Point2D{x: self.max.x.max(other.max.x), y: self.max.y.max(other.max.y)},
        }
    }
}
impl AddAssign<BoundingRect2D> for BoundingRect2D{
    fn add_assign(&mut self, rhs: BoundingRect2D) {
        self.min = Point2D{x: self.min.x.min(rhs.min.x), y: self.min.y.min(rhs.min.y)};
        self.min = Point2D{x: self.max.x.max(rhs.max.y), y: self.max.y.max(rhs.max.y)};
    }
}