///Vector that drops the oldest item when full
pub struct DropVec<T> {
    max_size: usize,
    index: usize,
    buffer: Vec<T>,
}

impl<T> DropVec<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            index: 0,
            buffer: Vec::with_capacity(max_size),
        }
    }
    pub fn push(&mut self, item: T) {
        if self.buffer.len() <= self.index {
            self.push(item);
        } else {
            self.buffer[self.index] = item;
        }
        self.index += 1;
        if self.index == self.max_size {
            self.index = 0;
        }
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = T>) {
        for item in iter.into_iter() {
            self.push(item);
        }
    }
}
