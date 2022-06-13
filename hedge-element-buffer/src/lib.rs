use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

pub type Tag = u32;
pub type Offset = u32;
pub type Generation = u32;

/// Our default value for uninitialized or unconnected components in the mesh.
pub const INVALID_ELEMENT_OFFSET: Offset = 0;
pub const INVALID_ELEMENT_GENERATION: Generation = 0;

/// Type-safe index into kernel storage.
#[derive(Default, Debug, Eq)]
pub struct Handle<T> {
    pub offset: Offset,
    pub generation: Generation,
    _marker: PhantomData<T>,
}

impl<T> Copy for Handle<T> {}
impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            offset: self.offset,
            generation: self.generation,
            _marker: self._marker,
        }
    }
}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.offset.hash(state);
        self.generation.hash(state);
    }
}

impl<T> Handle<T> {
    pub fn new(offset: Offset, generation: Generation) -> Handle<T> {
        Handle {
            offset,
            generation,
            _marker: PhantomData::default(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.offset != INVALID_ELEMENT_OFFSET
    }
}

impl<T> PartialOrd for Handle<T> {
    fn partial_cmp(&self, other: &Handle<T>) -> Option<Ordering> {
        // Only the offset should matter when it comes to ordering
        self.offset.partial_cmp(&other.offset)
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Handle<T>) -> bool {
        self.offset.eq(&other.offset) && self.generation.eq(&other.generation)
    }
}

/// A pretty simple wrapper over a pair of 'Vec's.
pub struct ElementBuffer<D: Default> {
    buffer: Vec<D>,
    generations: Vec<Generation>,
    // Why not put the index? Because the generation of an index could give us
    // false negatives if we're not careful ... I'm still considering this.
    free_cells: HashSet<Offset>,
    //tags: Vec<Tag>, // TODO: use a Set instead. This isn't a persistent array of attributes.
}

impl<D: Default> Default for ElementBuffer<D> {
    fn default() -> Self {
        ElementBuffer {
            buffer: vec![Default::default()],
            generations: vec![Default::default()],
            free_cells: HashSet::new(),
            //tags: Vec::new(),
        }
    }
}

impl<D: Default> fmt::Debug for ElementBuffer<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ElementBuffer<> {{ {} items }}", self.len())
    }
}

impl<D: Default> ElementBuffer<D> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let mut out = Self {
            buffer: Vec::with_capacity(capacity + 1),
            generations: Vec::with_capacity(capacity + 1),
            free_cells: HashSet::new(),
        };
        out.buffer.push(Default::default());
        out.generations.push(Default::default());
        out
    }

    #[inline(always)]
    fn is_active_cell(&self, offset: Offset) -> bool {
        !self.free_cells.contains(&offset)
    }

    /// Returns the number of currently active cells.
    /// The actual number of items allocated by the buffer might
    /// be different.
    pub fn len(&self) -> usize {
        (self.buffer.len() - 1) - self.free_cells.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn has_inactive_cells(&self) -> bool {
        !self.free_cells.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Handle<D>, &D)> {
        self.buffer
            .iter()
            .enumerate()
            .skip(1)
            .zip(self.generations.iter().skip(1))
            .filter(|((offset, _), _)| self.is_active_cell(*offset as Offset))
            .map(|((offset, element), generation)| {
                (Handle::new(offset as Offset, *generation), element)
            })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Handle<D>, &mut D)> {
        self.buffer
            .iter_mut()
            .enumerate()
            .skip(1)
            .zip(self.generations.iter().skip(1))
            .filter(|((offset, _), _)| {
                let offset = *offset as Offset;
                !self.free_cells.contains(&offset)
            })
            .map(|((offset, element), generation)| {
                (Handle::new(offset as Offset, *generation), element)
            })
    }

    pub fn get(&self, handle: Handle<D>) -> Option<&D> {
        if !self.is_active_cell(handle.offset) {
            return None;
        }

        let generation = self.generations[handle.offset as usize];
        if generation != handle.generation {
            return None;
        }

        self.buffer.get(handle.offset as usize)
    }

    pub fn get_mut(&mut self, handle: Handle<D>) -> Option<&mut D> {
        if !self.is_active_cell(handle.offset) {
            return None;
        }

        let generation = self.generations[handle.offset as usize];
        if generation != handle.generation {
            return None;
        }

        self.buffer.get_mut(handle.offset as usize)
    }

    pub fn push(&mut self, element: D) -> Handle<D> {
        if let Some(offset) = self.free_cells.iter().next().cloned() {
            self.free_cells.remove(&offset);
            // In this situation we just re-use an existing cell
            self.buffer[offset as usize] = element;
            Handle::new(offset, self.generations[offset as usize])
        } else {
            // Here we push the element on to the back
            let offset = self.buffer.len() as Offset;
            self.buffer.push(element);
            self.generations.push(1);
            Handle::new(offset, 1)
        }
    }

    pub fn remove(&mut self, handle: Handle<D>) {
        self.free_cells.insert(handle.offset);
        self.generations[handle.offset as usize] += 1;
    }

    // fn next_swap_pair(&self) -> Option<(Offset, Offset)> {
    //     let inactive_offset = self.enumerate().find(|e| !e.1.is_active()).map(|e| e.0);
    //     let active_offset = self
    //         .enumerate()
    //         .rev()
    //         .find(|e| e.1.is_active())
    //         .map(|e| e.0);
    //     if let (Some(inactive_offset), Some(active_offset)) = (inactive_offset, active_offset) {
    //         if active_offset < inactive_offset {
    //             log::debug!("Buffer appears to be successfully sorted!");
    //             // by the time this is true we should have sorted/swapped
    //             // all elements so that the inactive inactive elements
    //             // make up the tail of the buffer.
    //             None
    //         } else {
    //             Some((inactive_offset as u32, active_offset as u32))
    //         }
    //     } else {
    //         log::debug!("No more swap pairs.");
    //         None
    //     }
    // }
}

impl<D: Default> Index<Handle<D>> for ElementBuffer<D> {
    type Output = D;

    fn index(&self, handle: Handle<D>) -> &Self::Output {
        self.get(handle)
            .expect("Unable to retrieve item for provided handle.")
    }
}

impl<D: Default> IndexMut<Handle<D>> for ElementBuffer<D> {
    fn index_mut(&mut self, handle: Handle<D>) -> &mut Self::Output {
        self.get_mut(handle).expect("Unable to retrieve item.")
    }
}

///////////////////////////////////////////////////////////////////////////////

pub mod prelude {
    pub use super::{ElementBuffer, Generation, Handle, Offset, Tag};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestElement {
        foo: u32,
    }

    type TestHandle = Handle<TestElement>;
    type TestBuffer = ElementBuffer<TestElement>;

    #[test]
    fn default_index_is_invalid() {
        let index = TestHandle::default();
        assert!(!index.is_valid());
    }

    #[test]
    fn default_element_buffer_properties() {
        let buffer = TestBuffer::default();
        assert!(!buffer.has_inactive_cells());
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn freecells_set_test() {
        let mut foo: HashSet<u32> = HashSet::new();
        foo.insert(1);
        foo.insert(10);
        foo.insert(256);

        assert_eq!(foo.len(), 3);
        assert!(foo.iter().next().is_some());
        assert_eq!(foo.len(), 3);

        let offset = foo
            .iter()
            .next()
            .cloned()
            .expect("Failed to get first value from set.");
        assert!(foo.contains(&offset));
        foo.remove(&offset);
        assert!(!foo.contains(&offset));

        assert_eq!(foo.len(), 2);
    }

    #[test]
    fn push_elements() {
        let mut buffer = TestBuffer::default();

        assert!(buffer.is_empty());

        let i0 = buffer.push(TestElement { foo: 0 });
        let i1 = buffer.push(TestElement { foo: 1 });

        assert!(!buffer.is_empty());
        assert_eq!(buffer.len(), 2);

        assert_eq!(i0.offset, 1);
        assert_eq!(i0.generation, 1);

        assert_eq!(i1.offset, 2);
        assert_eq!(i1.generation, 1);
    }

    #[test]
    fn iter_elements() {
        let mut buffer = TestBuffer::default();
        let i0 = buffer.push(TestElement { foo: 0 });
        let i1 = buffer.push(TestElement { foo: 1 });
        let i2 = buffer.push(TestElement { foo: 2 });

        assert_eq!(buffer.iter().count(), 3);

        {
            let offsets: Vec<Offset> = buffer.iter().map(|(index, _)| index.offset).collect();
            assert_eq!(offsets.len(), 3);
            assert_eq!(offsets[0], i0.offset);
            assert_eq!(offsets[1], i1.offset);
            assert_eq!(offsets[2], i2.offset);
        }

        {
            let mut it = buffer.iter();

            let (_, element) = it.next().expect("Unexpected end of iterator");
            assert_eq!(element.foo, 0);

            let (_, element) = it.next().expect("Unexpected end of iterator");
            assert_eq!(element.foo, 1);

            let (_, element) = it.next().expect("Unexpected end of iterator");
            assert_eq!(element.foo, 2);

            assert!(it.next().is_none());
        }

        {
            for (_, element) in buffer.iter_mut() {
                element.foo += 1;
            }

            let foos: Vec<u32> = buffer.iter().map(|(_, e)| e.foo).collect();
            assert_eq!(foos[0], 1);
            assert_eq!(foos[1], 2);
            assert_eq!(foos[2], 3);
        }
    }

    #[test]
    fn indexing() {
        let mut buffer = TestBuffer::default();
        let i0 = buffer.push(TestElement { foo: 0 });
        let i1 = buffer.push(TestElement { foo: 1 });
        let i2 = buffer.push(TestElement { foo: 2 });

        assert_eq!(buffer[i0].foo, 0);
        assert_eq!(buffer[i1].foo, 1);
        assert_eq!(buffer[i2].foo, 2);

        buffer[i2].foo = 3;

        let foo_accum: u32 = buffer.iter().map(|(_, e)| e.foo).sum();
        assert_eq!(foo_accum, 4);
    }

    #[test]
    fn remove_elements() {
        let mut buffer = TestBuffer::default();
        let i0 = buffer.push(TestElement { foo: 0 });
        let i1 = buffer.push(TestElement { foo: 1 });
        let i2 = buffer.push(TestElement { foo: 2 });
        let i3 = buffer.push(TestElement { foo: 3 });
        let i4 = buffer.push(TestElement { foo: 4 });

        assert_eq!(buffer.len(), 5);

        buffer.remove(i2);
        buffer.remove(i3);

        assert_eq!(buffer.len(), 3);

        let foos: Vec<u32> = buffer.iter().map(|(_, e)| e.foo).collect();
        assert_eq!(foos[0], 0);
        assert_eq!(foos[1], 1);
        assert_eq!(foos[2], 4);

        let offsets: Vec<Offset> = buffer.iter().map(|(h, _)| h.offset).collect();
        assert_eq!(offsets[0], i0.offset);
        assert_eq!(offsets[1], i1.offset);
        assert_eq!(offsets[2], i4.offset);

        assert!(buffer.get(i2).is_none());
        assert!(buffer.get(i3).is_none());
    }

    #[test]
    fn insert_after_remove() {
        let mut buffer = TestBuffer::default();
        let i0 = buffer.push(TestElement { foo: 0 });
        let i1 = buffer.push(TestElement { foo: 1 });
        let i2 = buffer.push(TestElement { foo: 2 });
        let i3 = buffer.push(TestElement { foo: 3 });
        let i4 = buffer.push(TestElement { foo: 4 });

        assert_eq!(i0.generation, 1);
        assert_eq!(i1.generation, 1);
        assert_eq!(i2.generation, 1);
        assert_eq!(i3.generation, 1);
        assert_eq!(i4.generation, 1);

        assert_eq!(buffer.len(), 5);

        buffer.remove(i2);
        buffer.remove(i3);

        assert_eq!(buffer.len(), 3);

        let i5 = buffer.push(TestElement { foo: 5 });
        let i6 = buffer.push(TestElement { foo: 6 });

        assert_eq!(i5.generation, 2);
        assert_eq!(i6.generation, 2);

        assert_eq!(buffer.len(), 5);

        assert!(buffer.get(i2).is_none());
        assert!(buffer.get(i3).is_none());

        assert!(buffer.get(i5).is_some());
        assert!(buffer.get(i6).is_some());

        assert_eq!(buffer[i5].foo, 5);
        assert_eq!(buffer[i6].foo, 6);
    }
}
