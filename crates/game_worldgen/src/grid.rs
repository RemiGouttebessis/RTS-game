/// A 2D grid that wraps in X (the cylinder's circumference) and clamps in Y
/// (pole to pole — there's nothing past the poles to wrap to).
#[derive(Clone)]
pub struct Grid<T> {
    pub width: usize,
    pub height: usize,
    cells: Vec<T>,
}

impl<T: Clone + Default> Grid<T> {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![T::default(); width * height],
        }
    }
}

impl<T> Grid<T> {
    fn index(&self, x: i64, y: i64) -> usize {
        let x = x.rem_euclid(self.width as i64) as usize;
        let y = y.clamp(0, self.height as i64 - 1) as usize;
        y * self.width + x
    }

    pub fn get(&self, x: i64, y: i64) -> &T {
        &self.cells[self.index(x, y)]
    }

    pub fn set(&mut self, x: i64, y: i64, value: T) {
        let index = self.index(x, y);
        self.cells[index] = value;
    }

    pub fn iter_coords(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let width = self.width;
        (0..self.height).flat_map(move |y| (0..width).map(move |x| (x, y)))
    }

    /// The 8 neighbors of `(x, y)`, wrapped in X and clamped out at the poles
    /// (a cell on the polar edge simply has fewer neighbors).
    pub fn neighbors(&self, x: i64, y: i64) -> impl Iterator<Item = (i64, i64)> + '_ {
        const OFFSETS: [(i64, i64); 8] = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];
        OFFSETS.iter().filter_map(move |(dx, dy)| {
            let ny = y + dy;
            if ny < 0 || ny >= self.height as i64 {
                None
            } else {
                Some((x + dx, ny))
            }
        })
    }
}
