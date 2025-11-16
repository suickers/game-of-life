use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{
	window, 
	HtmlCanvasElement, 
	CanvasRenderingContext2d,
	HtmlButtonElement,
	MouseEvent,
};
use std::cell::RefCell;
use std::rc::Rc;

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
					ctx.set_fill_style(&JsValue::from_str("grey"));
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

	pub fn toggle_cell(&mut self, row: u32, col: u32) {
		let idx = self.get_index(row, col);
		self.cells[idx] = match self.cells[idx] {
			Cell::Alive => Cell::Dead,
			Cell::Dead => Cell::Alive,
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
		let width = 150;
		let height = 150;

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
}

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

	let uni_for_click = universe.clone();
	let canvas_for_click = canvas.clone();
	let cell_size_click = cell_size;

	let on_canvas_click = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
		let rect = canvas_for_click
			.dyn_ref::<web_sys::Element>()
			.unwrap()
			.get_bounding_client_rect();
		
		let scale_x = canvas_for_click.width() as f64 / rect.width();
		let scale_y = canvas_for_click.height() as f64 / rect.height();

		let canvas_x = (event.client_x() as f64 - rect.left()) * scale_x;
		let canvas_y = (event.client_y() as f64 - rect.top()) * scale_y;

		let col = (canvas_x / cell_size_click).floor() as u32;
		let row = (canvas_y / cell_size_click).floor() as u32;

		if row < uni_for_click.borrow().height && col < uni_for_click.borrow().width {
			uni_for_click.borrow_mut().toggle_cell(row, col);
		}
	}) as Box<dyn FnMut(_)>);

	canvas.set_onclick(Some(on_canvas_click.as_ref().unchecked_ref()));
	on_canvas_click.forget();
	
	let button = document
		.get_element_by_id("play-pause")
		.unwrap()
		.dyn_into::<HtmlButtonElement>()?;

	let running = Rc::new(RefCell::new(true));

	let running_for_click = running.clone();
	let button_for_click = button.clone();
	let on_click = Closure::wrap(Box::new(move |_e: MouseEvent| {
		let mut r = running_for_click.borrow_mut();
		*r = !*r;
		button_for_click.set_text_content(Some(if *r { "Pause" } else { "Play" }));
	}) as Box<dyn FnMut(_)>);
	
	button.set_onclick(Some(on_click.as_ref().unchecked_ref()));
	on_click.forget();

	
	let raf_handle: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
	let raf_handle_clone = raf_handle.clone();
	let uni_rc = universe.clone();
	let ctx_rc = ctx.clone();
	let mut frame_count = 0;
	let running_rc = running.clone();
	
	*raf_handle.borrow_mut() = Some(Closure::wrap(Box::new(move || {
		frame_count += 1;

		if *running_rc.borrow() && frame_count % 5 == 0 {
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
