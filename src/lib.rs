use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{window, HtmlCanvasElement, CanvasRenderingContext2d};
use std::cell::RefCell;
use std::rc::Rc;
// use std::fmt; 

#[wasm_bindgen]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
	Dead = 0,
	Alive = 1,
}

#[wasm_bindgen]
pub struct Universe {
	width: u32,
	height: u32,
	cells: Vec<Cell>,
}

impl Universe {
	fn get_index(&self, row: u32, column: u32) -> usize {
		(row * self.width + column) as usize
	}

	fn live_neighbour_count(&self, row: u32, column: u32) -> u8 {
		let mut count = 0;
		for delta_row in [self.height - 1, 0, 1].iter().cloned() {
			for delta_col in [self.width - 1, 0, 1].iter().cloned() {
				if delta_row == 0 && delta_col == 0 {
					continue;
				}

				let neighbour_row = (row + delta_row) % self.height;
				let neighbour_col = (column + delta_col) % self.width;
				let idx = self.get_index(neighbour_row, neighbour_col);
				count += self.cells[idx] as u8;
			}
		}
		count 
	}

	pub fn draw(&self, ctx: &CanvasRenderingContext2d, cell_size: f64) {
		for row in 0..self.height {
			for col in 0..self.width {
				let idx = self.get_index(row, col);

				if self.cells[idx] == Cell::Alive {
					ctx.set_fill_style(&JsValue::from_str("black"));
				} else {
					ctx.set_fill_style(&JsValue::from_str("white"));
				}
				ctx.fill_rect(
					(col as f64) * cell_size,
					(row as f64) * cell_size,
					cell_size,
					cell_size,
				);
			}
		}
	}
}

#[wasm_bindgen]
impl Universe {
	pub fn tick(&mut self) {
		let mut next = self.cells.clone();

		for row in 0..self.height {
			for col in 0..self.width {
				let idx = self.get_index(row, col);
				let cell = self.cells[idx];
				let live_neighbours = self.live_neighbour_count(row, col);

				let next_cell = match (cell, live_neighbours) {
					(Cell::Alive, x) if x < 2 => Cell::Dead,
					(Cell::Alive, 2) | (Cell::Alive, 3) => Cell::Alive, 
					(Cell::Alive, x) if x > 3 => Cell::Dead,
					(Cell::Dead, 3) => Cell::Alive,
					(otherwise, _) => otherwise,
				};

				next[idx] = next_cell;
			}
		}
		self.cells = next;
	}

	pub fn new() -> Self {
		let width = 64;
		let height = 64;

		let cells = (0..width * height)
			.map(|i| {
				if i % 2 == 0 || i % 7 == 0 {
					Cell::Alive
				} else {
					Cell::Dead
				}
			})
			.collect();

		Self {
			width,
			height,
			cells,
		}
	}

	// pub fn render(&self) -> String {
	// 	self.to_string()
	// }
}

// impl fmt::Display for Universe {
// 	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
// 		for line in self.cells.as_slice().chunks(self.width as usize) {
// 			for &cell in line {
// 				let symbol = if cell == Cell::Dead { '◻' } else { '◼' };
// 				write!(f, "{}", symbol)?;
// 			}
// 			write!(f, "/n")?;
// 		}
// 		Ok(())
// 	}
// }

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
	console_error_panic_hook::set_once();

	let document = window().unwrap().document().unwrap();
	let canvas = document
		.get_element_by_id("game-of-life-canvas")
		.unwrap()
		.dyn_into::<HtmlCanvasElement>()?;

	let universe = Universe::new();
	let cell_size: f64 = 8.0;

	canvas.set_width((universe.width * cell_size as u32) as u32);
	canvas.set_height((universe.height * cell_size as u32) as u32);

	let ctx = canvas 
		.get_context("2d")?
		.unwrap()
		.dyn_into::<CanvasRenderingContext2d>()?;

	let universe = Rc::new(RefCell::new(universe));
	let ctx = Rc::new(ctx);

	let raf_handle: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
	
	let raf_handle_clone = raf_handle.clone();
	let uni_rc = universe.clone();
	let ctx_rc = ctx.clone();

	let mut frame_count = 0;
	
	*raf_handle.borrow_mut() = Some(Closure::wrap(Box::new(move || {
		frame_count += 1;

		if frame_count % 15 == 0 {
			uni_rc.borrow_mut().tick();
		}
		
		
		let width = ctx_rc.canvas().unwrap().width() as f64;
		let height = ctx_rc.canvas().unwrap().height() as f64;
		ctx_rc.clear_rect(0.0, 0.0, width, height);
		uni_rc.borrow().draw(&ctx_rc, cell_size);

		
		let borrow = raf_handle_clone.borrow();
		let cb = borrow
			.as_ref()
			.unwrap()
			.as_ref()
			.unchecked_ref();
		
		window().unwrap().request_animation_frame(cb).unwrap();
		
	}) as Box<dyn FnMut()>));

	let borrow = raf_handle.borrow();
	let cb = borrow
		.as_ref()
		.unwrap()
		.as_ref()
		.unchecked_ref();

	window().unwrap().request_animation_frame(cb)?;



	Ok(())	
}
