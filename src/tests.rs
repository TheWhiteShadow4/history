#![allow(non_camel_case_types)]

use std::{cell::RefCell, rc::Rc};
use super::*;


#[derive(Debug, Clone, PartialEq, Default)]
struct Model1
{
	pub value1: i32,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct Model2
{
	pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct ApplicationState
{
	pub model1: Rc<RefCell<Model1>>,
	pub model2: Rc<RefCell<Model2>>,
}

impl ApplicationState
{
	pub fn new() -> Self
	{
		Self
		{
			model1: Rc::new(RefCell::new(Model1::default())),
			model2: Rc::new(RefCell::new(Model2::default())),
		}
	}
}

impl Stateful<SnapshotType> for ApplicationState
{
	fn state(&self, typ: &SnapshotType) -> &dyn Any
	{
		match typ
		{
			SnapshotType::Model_1 => &self.model1,
			SnapshotType::Model_2 => &self.model2,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum SnapshotType
{
	Model_1,
	Model_2,
}

pub struct Snapshot1
{
	data: Model1,
}

impl Snapshot1
{
	fn new(data: Model1) -> Self
	{
		Self { data }
	}
}

impl Snapshot for Snapshot1
{
	fn restore(&self, state: &dyn Any) -> Result<()>
	{
		let state = state.downcast_ref::<Rc<RefCell<Model1>>>().unwrap();
		*state.borrow_mut() = self.data.clone();
		println!("Model_1 Restore {}", state.borrow().value1);
		Ok(())
	}
}

pub struct Snapshot2
{
	data: Model2,
}

impl Snapshot2
{
	fn new(data: Model2) -> Self
	{
		Self { data }
	}
}

impl Snapshot for Snapshot2
{
	fn restore(&self, state: &dyn Any) -> Result<()>
	{
		let state = state.downcast_ref::<Rc<RefCell<Model2>>>().unwrap();
		*state.borrow_mut() = self.data.clone();
		println!("Model_2 Restore {:?}", state.borrow().data);
		Ok(())
	}
}

#[test]
fn basic_api_test()
{
	// Setup
	let state = ApplicationState::new();
	let mut history = History::new(state.clone(), 10);

	let model1 = state.model1.borrow_mut();
	history.begin(SnapshotType::Model_1, || Snapshot1::new(model1.clone()));
	drop(model1);

	let model2 = state.model2.borrow_mut();
	history.begin(SnapshotType::Model_2, || Snapshot2::new(model2.clone()));
	drop(model2);

	model1_edit(&mut history, &state, 42);
	model1_edit(&mut history, &state, 69);
	model2_edit(&mut history, &state, vec![1, 2, 3]);
	model1_edit(&mut history, &state, 117);

	history.undo().unwrap();
	assert_eq!(state.model1.borrow().value1, 69);
	assert_eq!(state.model2.borrow().data, vec![1, 2, 3]);

	history.undo().unwrap();
	assert_eq!(state.model1.borrow().value1, 69);
	assert_eq!(state.model2.borrow().data, vec![]);

	history.undo().unwrap();
	assert_eq!(state.model1.borrow().value1, 42);

	history.redo().unwrap();
	assert_eq!(state.model1.borrow().value1, 69);

	model1_edit(&mut history, &state, 11);
	assert_eq!(history.redo(), Err(HistoryError::NoSnapshot));
	history.undo().unwrap();
	assert_eq!(state.model1.borrow().value1, 69);
	history.undo().unwrap();
	assert_eq!(state.model1.borrow().value1, 42);
	history.undo().unwrap();
	assert_eq!(state.model1.borrow().value1, 0);
	assert_eq!(history.undo(), Err(HistoryError::NoSnapshot));

	history.redo().unwrap();
	history.redo().unwrap();
	assert_eq!(state.model1.borrow().value1, 69);
	assert_eq!(state.model2.borrow().data, vec![]);
}

fn model1_edit(history: &mut History<ApplicationState, SnapshotType>, state: &ApplicationState, new_val: i32)
{
	let mut testdata = state.model1.borrow_mut();
	let old = testdata.clone();
	history.begin(SnapshotType::Model_1, ||
	{
		panic!("Muss bereits gesetzt sein");
		#[allow(unreachable_code)]
		Snapshot1::new(testdata.clone())
	});

	// Edit
	println!("Editiere Model_1 {} => {}", testdata.value1, new_val);
	testdata.value1 = new_val;

	//Editor ende
	if old != *testdata
	{
		history.push(SnapshotType::Model_1, Snapshot1::new(testdata.clone())).unwrap();
	}
}

fn model2_edit(history: &mut History<ApplicationState, SnapshotType>, state: &ApplicationState, new_val: Vec<u8>)
{
	let mut testdata = state.model2.borrow_mut();
	let old = testdata.clone();
	history.begin(SnapshotType::Model_2, ||
	{
		panic!("Muss bereits gesetzt sein");
		#[allow(unreachable_code)]
		Snapshot2::new(testdata.clone())
	});

	// Edit
	println!("Editiere Model_2 {:?} => {:?}", testdata.data, new_val);
	testdata.data = new_val;

	//Editor ende
	if old != *testdata
	{
		history.push(SnapshotType::Model_2, Snapshot2::new(testdata.clone())).unwrap();
	}
}
