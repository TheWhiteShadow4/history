use std::ptr::{self, NonNull};


pub struct LoopedStack<T>
{
	buf: RawStack<T>,
	len: usize,
	offset: usize,
}

impl<T> LoopedStack<T>
{
	pub fn new(size: usize) -> Self
	{
		LoopedStack
		{
			buf: RawStack::new(size),
			len: 0,
			offset: 0,
		}
	}

	pub fn push(&mut self, value: T) -> Option<T>
	{
		if self.len < self.buf.size
		{
			self.push_new(value);
			None
		}
		else
		{
			Some(self.push_inplace(value))
		}
	}

	pub fn extend(&mut self, values: impl IntoIterator<Item = T>)
	{
		values.into_iter().for_each(|value| {self.push(value);});
	}

	pub fn get(&self, index: usize) -> Option<&T>
	{
		if index >= self.len
		{
			return None;
		}
		Some(unsafe
		{
			&*self.offset(index)
		})
	}

	pub fn peek(&self, nte: usize) -> Option<&T>
	{
		if nte >= self.len
		{
			return None;
		}
		let pos: usize = self.len - nte - 1;
		self.get(pos)
	}

	pub fn last(&self) -> Option<&T>
	{
		self.peek(0)
	}

	pub fn len(&self) -> usize
	{
		self.len
	}

	pub fn size(&self) -> usize
	{
		self.buf.size
	}

	pub fn pop(&mut self) -> Option<T>
	{
		if self.len == 0
		{
			return None;
		}
		self.len -= 1;
		Some(unsafe
		{
			ptr::read(self.cursor())
		})
	}

	fn push_new(&mut self, value: T)
	{
		unsafe
		{
			ptr::write(self.cursor(), value);
			self.len += 1;
        }
	}

	fn push_inplace(&mut self, value: T) -> T
	{
		let ret: T;
		unsafe
		{
			let ptr = self.cursor();
			ret = ptr::read(ptr);
			ptr::write(ptr, value);
			self.offset = (self.offset + 1) % self.buf.size;
		}
		ret
	}

	unsafe fn cursor(&self) -> *mut T
	{
		unsafe
		{
			self.as_mut_ptr().add((self.offset + self.len) % self.buf.size)
		}
	}

	unsafe fn offset(&self, offset: usize) -> *mut T
	{
		unsafe
		{
			self.as_mut_ptr().add((self.offset + offset) % self.buf.size)
		}
	}

	pub fn as_ptr(&self) -> *const T
	{
		self.buf.data.cast().as_ptr()
	}

	pub fn as_mut_ptr(&self) -> *mut T
	{
		self.buf.data.cast().as_ptr()
	}

	pub fn as_slice(&self) -> &[T]
	{
		unsafe
		{
			std::slice::from_raw_parts(self.as_mut_ptr(), self.buf.size)
		}
	}

	pub fn iter(&self) -> Iter<T>
	{
		Iter
		{
			buf: self.as_slice(),
			remain: self.len,
			offset: self.offset,
		}	
	}
}

impl<T> Drop for LoopedStack<T>
{
	fn drop(&mut self)
	{
		use ptr::drop_in_place as d;
		use ptr::slice_from_raw_parts_mut as slice;
		unsafe
		{
			if self.len < self.buf.size && self.offset != 0
			{
				if self.offset + self.len <= self.buf.size
				{	// Nur die hintere Hälfte ist belegt
					d(slice(self.as_mut_ptr().add(self.offset), self.len))
				}
				else
				{	// Wir haben eine Lücke in der Mitte und benötigen 2 Drops
					let len = self.buf.size - self.offset;
					d(slice(self.as_mut_ptr().add(self.offset), len));
					let len = self.len - len;
					d(slice(self.as_mut_ptr().add(0), len));
				}
			}
			else
			{	// Offset ist egal
				d(slice(self.as_mut_ptr(), self.len))
			}
		}
	}
}

struct RawStack<T>
{
	data: NonNull<u8>,
	size: usize,
	_marker: std::marker::PhantomData<T>,
}

impl<T> RawStack<T>
{
	fn layout(size: usize) -> std::alloc::Layout
	{
		std::alloc::Layout::from_size_align(size * std::mem::size_of::<T>(), std::mem::align_of::<T>()).unwrap()
	}

	fn new(size: usize) -> Self
	{
		let layout = Self::layout(size);
		let data = unsafe
		{
			let ptr = std::alloc::alloc(layout);
			NonNull::new_unchecked(ptr as *mut u8)
		};
		Self
		{
			data,
			size,
			_marker: std::marker::PhantomData,
		}
	}
}

impl<T> Drop for RawStack<T>
{
    fn drop(&mut self)
	{
		unsafe
		{
			std::alloc::dealloc(self.data.as_ptr(), Self::layout(self.size));
		}
    }
}

pub struct Iter<'a, T>
{
	buf: &'a [T],
	remain: usize,
	offset: usize,
}

impl<'a, T> Iterator for Iter<'a, T>
{
	type Item = &'a T;

	fn next(&mut self) -> Option<Self::Item>
	{
		if self.remain > 0
		{
			let val = &self.buf[self.offset];
			self.offset = (self.offset + 1) % self.buf.len();
			self.remain -= 1;
			Some(val)
		}
		else
		{
			None
		}
	}
}

impl<'a, T> DoubleEndedIterator for Iter<'_, T>
{
	fn next_back(&mut self) -> Option<Self::Item>
	{
		if self.remain > 0
		{
			self.remain -= 1;
			let val = &self.buf[(self.offset + self.remain) % self.buf.len()];
			Some(val)
		}
		else
		{
			None
		}
	}
}

#[cfg(test)]
mod tests
{
    use testdrop::TestDrop;

	#[test]
	fn test_looped_stack()
	{
		let mut stack = super::LoopedStack::new(3);

		assert_eq!(stack.offset, 0);
		assert_eq!(stack.len, 0);
		assert_eq!(stack.size(), 3);
		
		stack.push(1);
		assert_eq!(stack.offset, 0);
		assert_eq!(stack.len, 1);
		assert_eq!(stack.last(), Some(&1));

		stack.push(2);
		stack.push(3);

		// Stack ist voll. Länge ist damit 3 und Position ist wieder 0
		assert_eq!(stack.len, 3);
		assert_eq!(stack.offset, 0);
		assert_eq!(stack.peek(0), Some(&3));

		// Index 0 wird nun überschrieben
		stack.push(4);

		assert_eq!(stack.len, 3);
		assert_eq!(stack.offset, 1);

		assert_eq!(stack.get(0), Some(&2));
		assert_eq!(stack.get(1), Some(&3));
		assert_eq!(stack.get(2), Some(&4));

		// Prüfe get_prev für alle Elemente
		assert_eq!(stack.peek(0), Some(&4));
		assert_eq!(stack.peek(1), Some(&3));
		assert_eq!(stack.peek(2), Some(&2));
		assert_eq!(stack.peek(3), None);

		assert_eq!(stack.pop(), Some(4));
		assert_eq!(stack.len, 2);
		assert_eq!(stack.offset, 1);
		assert_eq!(stack.pop(), Some(3));
		assert_eq!(stack.pop(), Some(2));
		assert_eq!(stack.pop(), None);
		assert_eq!(stack.offset, 1);
		assert_eq!(stack.len, 0);
	}
	
	#[test]
	fn test_looped_stack_memory()
	{
		let td = TestDrop::new();
		{
			let mut stack = super::LoopedStack::new(3);
			stack.push(td.new_item().1);
			stack.push(td.new_item().1);
			stack.push(td.new_item().1);
			stack.push(td.new_item().1);
			stack.push(td.new_item().1);
			// Poppe ein Element um die Länge auf 2 zu reduzieren
			let vier = stack.pop().unwrap();
			td.assert_no_drop(4);
			drop(vier);

			// Der Stack hat die Belegung: [X][ ][X]

			td.assert_drop(0);
			td.assert_drop(1);
			td.assert_no_drop(2);
			td.assert_no_drop(3);
			td.assert_drop(4);
		}
		assert_eq!(td.num_dropped_items(), 5);
	}
}