use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    #[inline(always)]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline(always)]
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    #[inline(always)]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    #[inline(always)]
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    #[inline(always)]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    #[inline(always)]
    pub fn to_pos2(self) -> egui::Pos2 {
        egui::pos2(self.x, self.y)
    }

    #[inline(always)]
    pub fn to_vec2(self) -> egui::Vec2 {
        egui::vec2(self.x, self.y)
    }

    #[inline(always)]
    pub fn from_pos2(p: egui::Pos2) -> Self {
        Self::new(p.x, p.y)
    }

    #[inline(always)]
    pub fn from_vec2(v: egui::Vec2) -> Self {
        Self::new(v.x, v.y)
    }
}

#[inline(always)]
pub const fn vec2(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

impl From<(f32, f32)> for Vec2 {
    #[inline(always)]
    fn from(t: (f32, f32)) -> Self {
        Self::new(t.0, t.1)
    }
}

impl From<egui::Pos2> for Vec2 {
    #[inline(always)]
    fn from(p: egui::Pos2) -> Self {
        Self::new(p.x, p.y)
    }
}

impl From<egui::Vec2> for Vec2 {
    #[inline(always)]
    fn from(v: egui::Vec2) -> Self {
        Self::new(v.x, v.y)
    }
}

impl Add for Vec2 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Vec2 {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Vec2 {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl MulAssign<f32> for Vec2 {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl DivAssign<f32> for Vec2 {
    #[inline(always)]
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl Neg for Vec2 {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    #[inline(always)]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    #[inline(always)]
    pub fn left(&self) -> f32 {
        self.x.min(self.x + self.w)
    }

    #[inline(always)]
    pub fn right(&self) -> f32 {
        self.x.max(self.x + self.w)
    }

    #[inline(always)]
    pub fn top(&self) -> f32 {
        self.y.min(self.y + self.h)
    }

    #[inline(always)]
    pub fn bottom(&self) -> f32 {
        self.y.max(self.y + self.h)
    }

    #[inline(always)]
    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.left()
            && point.x <= self.right()
            && point.y >= self.top()
            && point.y <= self.bottom()
    }

    #[inline(always)]
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.left() <= other.right()
            && self.right() >= other.left()
            && self.top() <= other.bottom()
            && self.bottom() >= other.top()
    }

    #[inline(always)]
    pub fn to_egui(self) -> egui::Rect {
        egui::Rect::from_min_max(
            egui::pos2(self.left(), self.top()),
            egui::pos2(self.right(), self.bottom()),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    #[inline(always)]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[inline(always)]
    pub fn to_egui(self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.a.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }
}

pub const RED: Color = Color::new(1.0, 0.0, 0.0, 1.0);
pub const GREEN: Color = Color::new(0.0, 1.0, 0.0, 1.0);
pub const GRAY: Color = Color::new(0.5, 0.5, 0.5, 1.0);
