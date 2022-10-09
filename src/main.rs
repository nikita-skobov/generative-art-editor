use macroquad::prelude::*;
use egui_macroquad::egui;
use std::{collections::HashMap, ops::Index};

pub const BLOCK_WIDTH_PER_INPUT: f32 = 50.0;
pub const BLOCK_HEIGHT: f32 = 40.0;

pub fn screen_size() -> (f32, f32) {
    (screen_width(), screen_height())
}

pub struct JoinedSlice<'a> {
    pub a: Result<&'a [Input], &'a [&'a Input]>,
    pub b: &'a [Input],
}
impl<'a> JoinedSlice<'a> {
    pub fn new(a: &'a [Input], b: &'a [Input]) -> Self {
        Self { a: Ok(a), b }
    }
    pub fn new_ex(a: &'a [&'a Input], b: &'a [Input]) -> Self {
        Self { a: Err(a), b }
    }
}
impl<'a> Index<usize> for JoinedSlice<'a> {
    type Output = Input;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            match self.a {
                Ok(a) => if index < a.len() {
                    return a.get_unchecked(index)
                }
                Err(a) => if index < a.len() {
                    return a.get_unchecked(index)
                }
            }
            self.b.get_unchecked(index)
        }
    }
}

pub struct Input {
    pub name: String,
    pub value: f32, // TODO: have a value enum to support multiple value types
}

pub struct Block {
    pub inputs: Vec<Input>,
    pub num_outputs: usize,
    pub name: String,
    pub color: Color,
    pub run_fn: for<'a> fn(JoinedSlice<'a>, &[Block]),
}

pub struct BlockSet {
    pub blocks: Vec<Block>,
}

pub fn get_next_block(next_blocks: &[Block]) -> Option<(&Block, &[Block])> {
    if let Some(first_block) = next_blocks.first() {
        let next = if let Some(next) = next_blocks.get(1..) {
            next
        } else { &[] };
        Some((first_block, next))
    } else {
        None
    }
}

impl BlockSet {
    pub fn draw(&self, x: f32, mut y: f32) {
        for b in self.blocks.iter() {
            b.draw(x, y);
            y += BLOCK_HEIGHT;
        }
    }
    pub fn run(&self) {
        if let Some((first, next)) = get_next_block(&self.blocks) {
            let first_input = &first.inputs;
            let joined = JoinedSlice::new(first_input, &[]);
            (first.run_fn)(joined, next);
        }
    }
}

impl Block {
    pub fn draw(&self, x: f32, y: f32) {
        let num_parts = self.inputs.len().max(self.num_outputs);
        let total_width = num_parts as f32 * BLOCK_WIDTH_PER_INPUT;
        let orig_y = y;
        let orig_x = x;
        draw_rectangle(orig_x, orig_y, total_width, BLOCK_HEIGHT, self.color);
        let measured = measure_text(&self.name, None, 26, 1.0);
        let y = y + ((BLOCK_HEIGHT - measured.height) / 2.0);
        draw_text(&self.name, x + 2.0, y + measured.offset_y, 26.0, WHITE);
        let mut x = x + (BLOCK_WIDTH_PER_INPUT / 2.0);
        let triangle_width = 6.0;
        for i in 0..self.inputs.len() {
            let v1 = Vec2::new(x, orig_y);
            let v2 = Vec2::new(x + (triangle_width * 2.0), orig_y);
            let v3 = Vec2::new(x + triangle_width, orig_y + triangle_width);
            draw_triangle(v1, v2, v3, WHITE);
            let v1 = Vec2::new(x, orig_y + BLOCK_HEIGHT - triangle_width);
            let v2 = Vec2::new(x + (triangle_width * 2.0), orig_y + BLOCK_HEIGHT - triangle_width);
            let v3 = Vec2::new(x + triangle_width, orig_y + BLOCK_HEIGHT);
            draw_triangle(v1, v2, v3, WHITE);
            x += BLOCK_WIDTH_PER_INPUT;
        }
        draw_rectangle_lines(orig_x, orig_y, total_width, BLOCK_HEIGHT, 1.0, BLACK);
    }
}

pub struct Timeline {
    pub bar_pos: f32,
    pub max_height: f32,
    pub min_height: f32,
    /// a percentage (0 - 1) of how much vertical
    /// screen space to take up
    pub percentage_height: f32,
    /// must be at least 5s
    pub total_time_secs: f32,
    pub mark_height: f32,
}
impl Timeline {
    pub fn new(percentage_height: f32) -> Self {
        Self { bar_pos: 0.0, max_height: 300.0, min_height: 80.0, percentage_height, total_time_secs: 30.0, mark_height: 15.0 }
    }
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height;
        self
    }
    pub fn min_height(mut self, min_height: f32) -> Self {
        self.min_height = min_height;
        self
    }
    pub fn dimensions(&self) -> (f32, f32, f32, f32) {
        let (s_width, s_height) = screen_size();
        let mut height = s_height * self.percentage_height;
        if height > self.max_height {
            height = self.max_height;
        }
        if height < self.min_height {
            height = self.min_height;
        }
        let y = s_height - height;
        (0.0, y, s_width, height)
    }
    pub fn draw(&self) {
        let (x, y, w, h) = self.dimensions();
        draw_rectangle(x, y, w, h, BEIGE);
        let percentage_of_5s = 5.0 / self.total_time_secs;
        let width_per_5s = w * percentage_of_5s;
        let mut current_mark = 0.0;
        let mut current_time = 0;
        let s_height = screen_height();
        let mark_y = s_height - self.mark_height;
        while current_mark < w {
            draw_line(current_mark, mark_y, current_mark, s_height, 1.0, BLACK);
            draw_text(&format!("{current_time}s"), current_mark + 2.0, s_height - 2.0, 16.0, BLACK);
            current_time += 5;
            current_mark += width_per_5s;
        }
    }
}

pub struct EditorWindow {
    pub width: f32,
    pub bottom_margin: f32,
}
impl EditorWindow {
    pub fn new() -> Self {
        Self {
            width: 350.0,
            bottom_margin: 12.0,
        }
    }
    pub fn dimensions(&self, timeline: &Timeline) -> (f32, f32, f32, f32) {
        let s_width = screen_width();
        let (_, timeline_y, _, _) = timeline.dimensions();
        (s_width - self.width, 0.0, self.width, timeline_y - self.bottom_margin)
    }
    pub fn draw(&self, timeline: &Timeline) {
        let (x, y, w, h) = self.dimensions(timeline);
        egui_macroquad::ui(|egui_ctx| {
            let mut visuals = egui::Visuals::dark();
            visuals.window_shadow.extrusion = 0.0;
            visuals.popup_shadow.extrusion = 0.0;
            egui_ctx.set_visuals(visuals);
            egui::Window::new("")
                .collapsible(false)
                .title_bar(false)
                .fixed_size((w, h))
                .fixed_pos((x, y))
                .default_size((w, h))
                .resizable(false)
                .show(egui_ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.label("sdas");
                        });
                });
        });
    }
}

fn run_grid<'a>(input: JoinedSlice<'a>, next_blocks: &[Block]) {
    let (first, next) = if let Some(x) = get_next_block(next_blocks) {
        x
    } else { return };

    let rows = input[0].value;
    let cols = input[1].value;
    let (s_width, s_height) = get_screen_space();
    let height_per_row = s_height / rows;
    let width_per_col = s_width / cols;
    let rows = rows as u32;
    let cols = cols as u32;
    let mut y = height_per_row / 2.0;
    for _ in 0..rows {
        let mut x = width_per_col / 2.0;
        for _ in 0..cols {
            let inputs = [Input { name: "".into(), value: x }, Input { name: "".into(), value: y }];
            let first_inputs = &first.inputs[..];
            let joined = JoinedSlice::new(&inputs, first_inputs);
            (first.run_fn)(joined, next);
            x += width_per_col;
        }
        y += height_per_row;
    }
}

fn run_circle<'a>(input: JoinedSlice<'a>, next_blocks: &[Block]) {
    // macroquad::logging::info!("Circle w/ radius: {}", input[2].value);
    draw_circle_lines(input[0].value, input[1].value, input[2].value, 3.0, RED);
}

fn run_pass_time2<'a>(input: JoinedSlice<'a>, next_blocks: &[Block]) {
    let (first, next) = if let Some(x) = get_next_block(next_blocks) {
        x
    } else {
        return
    };

    // TODO: it shouldnt just get the total time, but rather the time since
    // this block has started rendering
    let time = get_time() as f32;
    let time_input = [
        Input { name: "".into(), value: 0.0 },
        Input { name: "".into(), value: 0.0 },
        Input { name: "".into(), value: time }
    ];
    let previous_inputs = [
        &input[0],
        &input[1]
    ];
    let joined = JoinedSlice::new_ex(&previous_inputs, &time_input);
    (first.run_fn)(joined, next);
}

static mut SCREEN_SPACE: (f32, f32) = (0.0, 0.0);
fn set_screen_space(s: (f32, f32)) {
    unsafe {
        SCREEN_SPACE = s;
    }
}
fn get_screen_space() -> (f32, f32) {
    unsafe { SCREEN_SPACE }
}

#[macroquad::main("BasicShapes")]
async fn main() {
    let window = EditorWindow::new();
    let timeline = Timeline::new(0.25);
    let b = Block {
        inputs: vec![
            Input { name: "rows".into(), value: 10.0 },
            Input { name: "cols".into(), value: 10.0 },
        ],
        num_outputs: 2,
        run_fn: run_grid,
        name: "Grid".into(),
        color: ORANGE,
    };
    let b1 = Block {
        name: "PassTime2".into(),
        color: ORANGE,
        inputs: vec![
            Input { name: "a".into(), value: 0.0 },
            Input { name: "b".into(), value: 0.0 },
        ],
        num_outputs: 3,
        run_fn: run_pass_time2,
    };
    let b2 = Block {
        inputs: vec![
            Input { name: "cx".into(), value: 0.0 },
            Input { name: "cy".into(), value: 0.0 },
            Input { name: "radius".into(), value: 10.0 },
        ],
        num_outputs: 0,
        run_fn: run_circle,
        name: "Circle".into(),
        color: BLUE,
    };
    let block_set = BlockSet {
        blocks: vec![b, b1, b2],
    };
    loop {
        clear_background(WHITE);

        let (x, _, _, h) = window.dimensions(&timeline);
        set_screen_space((x, h));
        block_set.run();
        
        window.draw(&timeline);
        // the timeline + art gets rendered below
        timeline.draw();
        block_set.draw(100.0, 100.0);

        // egui gets rendered on top
        egui_macroquad::draw();
        next_frame().await
    }
}
