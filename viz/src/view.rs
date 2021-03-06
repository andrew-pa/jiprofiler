use std::fmt;
use std::error::Error;

use runic::*;
use winit::*;

use data::*;
use menu::*;

pub struct Resources {
    pub font: Font
}

impl Resources {
    pub fn init(rx: &mut RenderContext) -> Result<Resources,Box<Error>> {
        Ok(Resources {
            font: rx.new_font("Consolas", 16.0, FontWeight::Regular, FontStyle::Normal)?
        })
    }
}

pub trait VizView {
    fn event(&mut self, e: &WindowEvent, data: &VizData, menus: &mut MenuContext) -> bool;
    fn menu_selection(&mut self, data: &VizData, tag: &'static str, sel: usize);
    fn reset(&mut self);

    fn paint(&mut self, rx: &mut RenderContext, res: &Resources, data: &VizData);

    fn status(&self, data: &VizData) -> String;
}

pub struct FlameChart {
    current_thread_id: usize,
    offset_x: i64,
    pixels_per_nanosecond: f32,
    last_mouse: Point,
    mouse_state: Option<(MouseButton, Point, i64)>,
    bounds: Rect,
    selected_index: isize,
}

impl FlameChart {
    pub fn init(rx: &mut RenderContext) -> FlameChart {
        FlameChart {
            current_thread_id: 0,
            offset_x: 0,
            pixels_per_nanosecond: 0.00005,
            last_mouse: Point::xy(0.0, 0.0), mouse_state: None,
            bounds: rx.bounds(),
            selected_index: -1,
        }
    }
}

impl VizView for FlameChart {
    fn status(&self, data: &VizData) -> String {
        let current_thread_ix = if self.current_thread_id == 0 { 0 } else { data.thread_ids[self.current_thread_id-1] };
        format!("{}, {:.2}% | Thread #{}", self.offset_x, ((self.offset_x) as f64 / data.abs_end_time as f64)*100.0,
                current_thread_ix)
    }

    fn reset(&mut self) {
        self.offset_x = 0;
        self.pixels_per_nanosecond = 0.0;
    }

    fn event(&mut self, e: &WindowEvent, data: &VizData, menus: &mut MenuContext) -> bool {
        match e {
            &WindowEvent::KeyboardInput { input: k, .. } => {
                match k.virtual_keycode {
                    Some(VirtualKeyCode::Left) => {
                        self.offset_x -= ((self.bounds.w * 0.1) / self.pixels_per_nanosecond) as i64; 
                    },
                    Some(VirtualKeyCode::Right) => {
                        self.offset_x += ((self.bounds.w * 0.1) / self.pixels_per_nanosecond) as i64;
                    }
                    Some(VirtualKeyCode::Up) => {
                        //self.pixels_per_nanosecond -= 0.000001;
                        self.pixels_per_nanosecond *= 0.9;
                    },
                    Some(VirtualKeyCode::Down) => {
                        //self.pixels_per_nanosecond += 0.000001;
                        self.pixels_per_nanosecond /= 0.9;
                    },
                    Some(VirtualKeyCode::PageUp) => {
                        if k.state == ElementState::Released {
                            if self.current_thread_id < data.thread_ids.len() {
                                self.current_thread_id += 1;
                            }
                        }
                    },
                    Some(VirtualKeyCode::PageDown) => {
                        if k.state == ElementState::Released {
                            self.current_thread_id = self.current_thread_id.saturating_sub(1);
                        }
                    }
                    _ => {}
                }
            },
            &WindowEvent::CursorMoved { position: (x,y), .. } => {
                if let Some((MouseButton::Left, click_pos, click_offset)) = self.mouse_state {
                    self.offset_x = ((click_pos.x - self.last_mouse.x) / self.pixels_per_nanosecond) as i64 + click_offset;
                }
                self.last_mouse = Point::xy(x as f32, y as f32);
            },
            &WindowEvent::MouseInput{ state, button, .. } => {
                self.mouse_state = match state {
                    ElementState::Pressed =>
                        Some((button, self.last_mouse, self.offset_x)),
                    _ => None
                };
                if state == ElementState::Released && button == MouseButton::Right {
                    let current_thread_ix = if self.current_thread_id == 0 { 0 } else { data.thread_ids[self.current_thread_id-1] };
                    self.selected_index = -1;
                    for (i,cr) in data.calls.iter().enumerate() {
                        if current_thread_ix > 0 && cr.thread_id != current_thread_ix { continue; }
                        let w = cr.elapsed_time as f32 * self.pixels_per_nanosecond;
                        if w < 2.0 { continue; }
                        let x = (-self.offset_x + (cr.start_time) as i64) as f32 * self.pixels_per_nanosecond;
                        if x+w < 0.0 || x > self.bounds.w { continue; }

                        if Rect::xywh(x, 34.0*cr.depth as f32, w, 32.0).contains(self.last_mouse) {
                            menus.popup(vec![ "zoom into view" ], self.last_mouse, "call");
                            self.selected_index = i as isize;
                            return true;
                        }
                    }
                }
            },
            &WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        self.pixels_per_nanosecond += y * 0.00001;
                    },
                    MouseScrollDelta::PixelDelta(x, y) => {
                        self.pixels_per_nanosecond += y * 0.0001;
                    }
                }
            },
            _ => {}

        }
        false
    }

    fn menu_selection(&mut self, data: &VizData, tag: &'static str, sel: usize) {
        if tag == "call" {
            match sel {
                0 => {
                    let cr = &data.calls[self.selected_index as usize];
                    let (new_offset, new_ppn) = (cr.start_time as i64, self.bounds.w / cr.elapsed_time as f32);
                    self.pixels_per_nanosecond = (self.bounds.w / cr.elapsed_time as f32) * 0.9;
                    self.offset_x = cr.start_time as i64 - (self.bounds.w * 0.05 / self.pixels_per_nanosecond) as i64;
                }
                _ => unreachable!()
            }
        }
    }

    fn paint(&mut self, rx: &mut RenderContext, res: &Resources, data: &VizData) {
        self.pixels_per_nanosecond = self.pixels_per_nanosecond.max(0.000001);
        self.offset_x = self.offset_x.max(0);
        self.bounds = rx.bounds();
        let current_thread_ix = if self.current_thread_id == 0 { 0 } else { data.thread_ids[self.current_thread_id-1] };

        let mut hovered_record: Option<&CallRecord> = None;

        for cr in data.calls.iter() {
            if current_thread_ix > 0 && cr.thread_id != current_thread_ix { continue; }
            let w = cr.elapsed_time as f32 * self.pixels_per_nanosecond;
            if w < 2.0 { continue; }
            let x = (-self.offset_x + (cr.start_time) as i64) as f32 * self.pixels_per_nanosecond;
            if x+w < 0.0 || x > self.bounds.w { continue; }

            rx.set_color(Color::rgb(0.8, 0.6, (cr.method_id as f32 * 8.23).sin().abs()));
            let r = Rect::xywh(x, 34.0*cr.depth as f32, w, 32.0);
            rx.fill_rect(r);
            if r.contains(self.last_mouse) {
                hovered_record = Some(cr);
                rx.set_color(Color::rgb(0.6, 0.2, (cr.method_id as f32 * 8.23).sin().abs()));
            } else {
                rx.set_color(Color::rgb(0.2, 0.4, (cr.method_id as f32 * 8.23).sin().abs()));
            }
            rx.stroke_rect(r, 2.0);
            if w > 128.0 {
                rx.set_color(Color::rgb(0.0, 0.0, 0.0));
                match data.method_index.get(&cr.method_id) {
                    Some(m) => {
                        let tr = Rect::xywh(r.x.max(0.0) + 2.0, r.y + 2.0, r.w, r.h);
                        rx.draw_text(tr, m, &res.font)
                    },
                    None => {}
                }
            }
        }

        // draw tooltip
        if let Some(cr) = hovered_record {
            let tx = rx.new_text_layout(&format!("{}\nStart Time: {}ns\nElapsed Time: {}ns\nThread #{}, Depth {}",
                                                 data.method_index.get(&cr.method_id).unwrap_or(&String::from("?")),
                                                 cr.start_time, cr.elapsed_time, cr.thread_id, cr.depth),
                                        &res.font, self.bounds.w, self.bounds.h).expect("create tooltip layout");
            rx.set_color(Color::rgb(0.3, 0.3, 0.3));
            let mut ttb = tx.bounds().offset(self.last_mouse).offset(Point::xy(16.0, 0.0));
            ttb.w += 8.0; ttb.h += 8.0;
            rx.fill_rect(ttb);
            rx.set_color(Color::rgb(0.8, 0.8, 0.8));
            rx.draw_text_layout(Point::xy(4.0 + ttb.x, 4.0 + ttb.y), &tx);
            rx.set_color(Color::rgb(0.6, 0.6, 0.6));
            rx.stroke_rect(ttb, 2.0);
        }
    }
}
