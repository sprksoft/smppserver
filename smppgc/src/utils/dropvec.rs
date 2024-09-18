use std::mem::MaybeUninit;

struct Slot<T> {
    init: bool,
    item: MaybeUninit<T>,
}
impl<T> Slot<T> {
    pub fn replace(&mut self, new: T) {
        if self.init {
            unsafe { self.item.assume_init_read() };
        }
        self.item.write(new);
        self.init = true;
    }
}
impl<T> Default for Slot<T> {
    fn default() -> Self {
        Self {
            init: false,
            item: MaybeUninit::uninit(),
        }
    }
}

pub struct Iter<'a, T> {
    index: usize,
    start_index: usize,
    drop_vec: &'a DropVec<T>,
}
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.index += 1;
            if self.index == self.drop_vec.buffer.len() {
                self.index = 0;
            }
            if self.index == self.start_index {
                return None;
            };
            let slot = &self.drop_vec.buffer[self.index];
            if slot.init {
                return Some(unsafe { slot.item.assume_init_ref() });
            };
        }
    }
}

///Vector that drops the oldest item when full
pub struct DropVec<T> {
    index: usize,
    buffer: Box<[Slot<T>]>,
}
impl<T> DropVec<T> {
    pub fn new(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffer.push(Slot::default());
        }
        Self {
            index: 0,
            buffer: buffer.into_boxed_slice(),
        }
    }
    pub fn push(&mut self, item: T) {
        self.buffer[self.index].replace(item);
        self.index += 1;
        if self.index == self.buffer.len() {
            self.index = 0;
        }
    }

    pub fn extend(&mut self, iter: impl IntoIterator<Item = T>) {
        for item in iter.into_iter() {
            self.push(item);
        }
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            index: self.index,
            start_index: self.index,
            drop_vec: self,
        }
    }
}
