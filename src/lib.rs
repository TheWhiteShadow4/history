use std::{any::Any, collections::HashMap};

use stack::LoopedStack;

pub mod stack;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HistoryError
{
	NoSnapshot,
	RestoreFailed,
}

pub type Result<T> = std::result::Result<T, HistoryError>;

/// Eine Historie zur Realisierung von Undo/Redo Funktionen.
/// 
/// Die Historie speichert Snapshots zu ein oder mehreren Modellen in einer Chronologischen Liste.
/// Die Liste ist als Ringpuffer implementiert, sodass die ältesten Einträge überschrieben werden.
/// Dabei bleibt immer der älteste Snapshot eines Modells erhalten,
/// so dass die Anzahl an nachfolgenden Undo Aktionen gleich der Kapazität der Historie ist.
/// 
/// - Dabei gibt `T` den Identifier für das jeweilige Modell an. z.B. eine Enum.
/// - `S` kann als Zugriffsobjekt auf die Modelle verwendet werden um sie im Snapshot nutzen zu können.
/// 
/// Es ist möglich nicht zurückgerollte Undos zu verwerfen oder als Zwischenschritte zu behalten.
pub struct History<S, T>
where
	T: PartialEq + Eq + std::hash::Hash + Clone,
	S: Stateful<T>
{
	inner: InnerHistory<S, T>,
}

impl<S, T> History<S, T>
where
T: PartialEq + Eq + std::hash::Hash + Clone,
	S: Stateful<T>
{
	pub fn new(stateful: S, capacity: usize) -> Self
	{
		debug_assert!(capacity > 0);
		Self
		{
			inner: InnerHistory::new(stateful, capacity),
		}
	}

	pub fn capacity(&self) -> usize
	{
		self.inner.entries.size()
	}

	pub fn size(&self) -> usize
	{
		self.inner.entries.len()
	}

	/// Initialisiert die Historie für ein Modell und erstellt bei Bedarf den ersten Snapshot.
	pub fn begin<F, U>(&mut self, typ: T, f: F)
	where
		U: Snapshot + 'static,
		F: FnOnce() -> U
	{
		let inner = &mut self.inner;
		if !inner.baselines.contains_key(&typ)
		{
			let baseline = f();
			inner.baselines.insert(typ, Box::new(baseline));
		}
	}

	/// Fügt einen neuen Zustand zur Historie hinzu.
	/// Hierbei werden nicht zurückgerollte Undo Schritte gelöscht.
	pub fn push<U>(&mut self, typ: T, snapshot: U) -> Result<()>
	where U: Snapshot + 'static,
	{
		let inner = &mut self.inner;
		inner.push(Entry
		{
			typ,
			snaps: Box::new(snapshot),
		});
		Ok(())
	}

	/// Fügt einen neuen Zustand zur Historie hinzu.
	/// Hierbei bleiben nicht zurückgerollte Undo Schritte erhalten.
	pub fn insert<U>(&mut self, typ: T, snapshot: U) -> Result<()>
	where U: Snapshot + 'static,
	{
		let inner = &mut self.inner;
		inner.insert(Entry
		{
			typ,
			snaps: Box::new(snapshot),
		});
		Ok(())
	}

	#[inline]
	pub fn can_undo(&self) -> bool
	{
		self.inner.can_undo()
	}

	#[inline]
	pub fn can_redo(&self) -> bool
	{
		self.inner.can_redo()
	}

	pub fn undo(&mut self) -> Result<()>
	{
		if let Some((typ, snaps)) = self.get_last_snapshot()
		{
			let ref state = *self.inner.stateful.state(typ);
			snaps.restore(state)?;
			self.inner.undo();
			Ok(())
		}
		else
		{
			Err(HistoryError::NoSnapshot)
		}
	}

	pub fn redo(&mut self) -> Result<()>
	{
		if let Some(entry) = self.inner.undo_stack.pop()
		{
			entry.restore(&self.inner.stateful)?;
			self.inner.redo(entry);
			Ok(())
		}
		else
		{
			Err(HistoryError::NoSnapshot)
		}
	}

	pub fn get_last_snapshot(&self) -> Option<(&T, &Box<dyn Snapshot>)>
	{
		let inner = &self.inner;
		let mut iter = inner.entries.iter().rev();
		let last = iter.next();
		if last.is_none() {return None;}

		let snapshot_typ = &last.unwrap().typ;
		for entry in iter
		{
			if entry.typ == *snapshot_typ
			{
				return Some((snapshot_typ, &entry.snaps));
			}
		}
		inner.baselines.get(snapshot_typ).map(|s| (snapshot_typ, s))
	}
}

struct InnerHistory<S, T>
where T: PartialEq, S: Stateful<T>
{
	entries: LoopedStack<Entry<T>>,
	undo_stack: Vec<Entry<T>>,
	baselines: HashMap<T, Box<dyn Snapshot>>,
	stateful: S,
}

impl<S, T> InnerHistory<S, T>
where T: PartialEq, S: Stateful<T>
{
	pub fn new(stateful: S, capacity: usize) -> Self
	{
		InnerHistory
		{
			entries: LoopedStack::new(capacity),
			undo_stack: Vec::with_capacity(capacity),
			baselines: HashMap::new(),
			stateful,
		}
	}

	fn push(&mut self, entry: Entry<T>) -> Option<Entry<T>>
	{
		// Push verwift alle nachfolgenden Undo Schritte
		self.undo_stack.clear();
		self.entries.push(entry)
	}

	fn insert(&mut self, entry: Entry<T>) -> Option<Entry<T>>
	{
		// Insert fügt alle nachfolgenden Undos als Zwischenschritte wieder ein 
		self.entries.extend(self.undo_stack.drain(..));
		self.entries.push(entry)
	}

	#[inline]
	fn can_undo(&self) -> bool
	{
		self.entries.len() > 0
	}

	#[inline]
	fn can_redo(&self) -> bool
	{
		self.undo_stack.len() > 0
	}

	fn undo(&mut self)
	{
		if let Some(entry) = self.entries.pop()
		{
			self.undo_stack.push(entry);
		}
	}

	fn redo(&mut self, entry: Entry<T>)
	{
		let old = self.entries.push(entry);
		// Der Stack kann nicht überlaufen.
		// Es gibt immer so viele frei Einträge wie im Undo-Stack.
		debug_assert!(old.is_none());
	}
}

pub struct Entry<T>
{
	typ: T,
	snaps: Box<dyn Snapshot>,
}

impl<T> Entry<T>
{
	pub fn new(typ: T, snaps: Box<dyn Snapshot>) -> Self
	{
		Entry { typ, snaps }
	}

	pub fn restore(&self, stateful: &dyn Stateful<T>) -> Result<()>
	{
		self.snaps.restore(stateful.state(&self.typ))
	}
}

/// Ein Trait um den Zustand eines Objekts zu setzen.
pub trait Snapshot
{
	fn restore(&self, state: &dyn Any) -> Result<()>;
}

/// Ein Trait um ein Datenmodel zu einem Stack Typ zu erhalten.
pub trait Stateful<T>
{
	fn state(&self, typ: &T) -> &dyn Any;
}
