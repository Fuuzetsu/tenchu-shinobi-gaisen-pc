use std::ops::{Index, IndexMut};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vector3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

pub type Position = Vector3<i32>;

impl<T> Vector3<T> {
    pub const fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }

    pub fn from_fn(mut f: impl FnMut(usize) -> T) -> Self {
        Self::new(f(0), f(1), f(2))
    }

    pub fn to_array(self) -> [T; 3] {
        [self.x, self.y, self.z]
    }

    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Vector3<U> {
        Vector3::new(f(self.x), f(self.y), f(self.z))
    }

    pub fn iter(&self) -> std::array::IntoIter<&T, 3> {
        [&self.x, &self.y, &self.z].into_iter()
    }

    pub fn iter_mut(&mut self) -> std::array::IntoIter<&mut T, 3> {
        [&mut self.x, &mut self.y, &mut self.z].into_iter()
    }
}

impl Position {
    pub const ZERO: Self = Self::new(0, 0, 0);

    pub fn wrapping_add(self, other: Self) -> Self {
        Self::new(
            self.x.wrapping_add(other.x),
            self.y.wrapping_add(other.y),
            self.z.wrapping_add(other.z),
        )
    }

    pub fn wrapping_sub(self, other: Self) -> Self {
        Self::new(
            self.x.wrapping_sub(other.x),
            self.y.wrapping_sub(other.y),
            self.z.wrapping_sub(other.z),
        )
    }
}

impl<T> From<[T; 3]> for Vector3<T> {
    fn from([x, y, z]: [T; 3]) -> Self {
        Self::new(x, y, z)
    }
}

impl<T> From<Vector3<T>> for [T; 3] {
    fn from(value: Vector3<T>) -> Self {
        value.to_array()
    }
}

impl<T> Index<usize> for Vector3<T> {
    type Output = T;
    fn index(&self, axis: usize) -> &T {
        match axis {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("Vector axis {axis} outside 0..3"),
        }
    }
}

impl<T> IndexMut<usize> for Vector3<T> {
    fn index_mut(&mut self, axis: usize) -> &mut T {
        match axis {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            _ => panic!("Vector axis {axis} outside 0..3"),
        }
    }
}

impl<T> IntoIterator for Vector3<T> {
    type Item = T;
    type IntoIter = std::array::IntoIter<T, 3>;
    fn into_iter(self) -> Self::IntoIter {
        self.to_array().into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Vector3<T> {
    type Item = &'a T;
    type IntoIter = std::array::IntoIter<&'a T, 3>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Vector3<T> {
    type Item = &'a mut T;
    type IntoIter = std::array::IntoIter<&'a mut T, 3>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T: PartialEq> PartialEq<[T; 3]> for Vector3<T> {
    fn eq(&self, other: &[T; 3]) -> bool {
        self.x == other[0] && self.y == other[1] && self.z == other[2]
    }
}
