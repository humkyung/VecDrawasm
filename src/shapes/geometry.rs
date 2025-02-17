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