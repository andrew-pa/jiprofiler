#![feature(conservative_impl_trait)]
extern crate runic;
extern crate winit;
extern crate futures;
extern crate futures_cpupool;
extern crate zip;

use std::io;
use std::io::{BufRead, BufReader};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::iter::{FromIterator, repeat};

use runic::*;
use winit::*;

use std::thread;
use std::thread::Thread;
use std::sync::{Arc, RwLock, TryLockError};
use std::sync::atomic::{AtomicBool, Ordering};

mod data;
use data::{VizData};
mod view;
use view::*;

mod menu;
use menu::*;

struct VizApp {
    data: Arc<RwLock<VizData>>,
    res: Resources,
    view: Box<VizView>,
    mx: MenuContext,
    last_mouse: Point
}

impl VizApp {
    fn init(rx: &mut RenderContext) -> VizApp {
        let mut args = std::env::args().skip(1);
        let data = Arc::new(RwLock::new(args.next().map(|perf_path| VizData::new(perf_path)).unwrap_or_default()));
        if data.read().unwrap().path.is_some() {
            let tdata = data.clone();
            let t = thread::spawn(move || { VizData::load(tdata).expect("load viz data"); });
        }
        let res = Resources::init(rx).expect("create graphics resources");
        VizApp {
            data: data,
            res: res,
            view: Box::new(FlameChart::init(rx)),
            mx: MenuContext::new(),
            last_mouse: Point::default(),
        }
    }
}

impl App for VizApp {
    fn paint(&mut self, rx: &mut RenderContext) {
        rx.clear(Color::rgb(0.1, 0.1, 0.11));

        match self.data.try_read() {
            Ok(d) => {
                let bounds = rx.bounds();
                let status_text = match d.path.as_ref() {
                    Some(p) => format!("{} | {} records {}[{}]",
                                       self.view.status(&d),
                                       d.calls.len(),
                                       if !d.loaded { "[still loading...] " } else { "" },
                                       d.path.as_ref().unwrap().display()),
                    None => String::from("no file")
                };
                let status_tx = rx.new_text_layout(&status_text,
                                                   &self.res.font, bounds.w, bounds.h).expect("create status text layout");
                rx.set_color(Color::rgb(0.3, 0.3, 0.3));
                rx.fill_rect(Rect::xywh(0.0, 0.0, bounds.w, status_tx.bounds().h+2.0));
                rx.set_color(Color::rgb(0.8, 0.8, 0.8));
                rx.draw_text_layout(Point::xy(2.0, 0.0), &status_tx);
                self.view.paint(rx, &self.res, &d);
                self.mx.paint(rx, &self.res);
            },
            Err(TryLockError::WouldBlock) => {
                rx.set_color(Color::rgb(0.7, 0.7, 0.7));
                rx.draw_text(Rect::xywh(32.0, 32.0, 1000.0, 1000.0), "loading...", &self.res.font);
            },
            Err(TryLockError::Poisoned(e)) => {
                panic!("poisoned lock {}", e);
            }
        } 
    }

    fn event(&mut self, ev: Event) -> bool {
        if let Event::WindowEvent { event: e, .. } = ev {
            match e {
                WindowEvent::MouseMoved { position, .. } => {
                    self.last_mouse = Point::from(position);
                },
                _ => {}
            }
            let d = self.data.read().unwrap();
            match self.mx.event(&e) {
                Some(("main", i)) => match i {
                    0 => {
                    },
                    1 => {
                        self.view.reset();
                    },
                    _ => {}
                },
                Some((tag, i)) => self.view.menu_selection(&d, tag, i),
                None => {}
            }
            if !self.view.event(&e, &d, &mut self.mx) {
                if let WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Right, .. } = e {
                    self.mx.popup(vec![ "load file", "reset view" ], self.last_mouse, "main");
                }
            }
        }
        false
    }
}

fn main() {
    runic::init();
    let mut evl = EventsLoop::new();
    let mut window = WindowBuilder::new()
        .with_dimensions(512, 521)
        .with_title("Java Performance Visualizer")
        .build(&evl).expect("create window!");
    let mut rx = RenderContext::new(&mut window).expect("create render context!");
    let mut app = VizApp::init(&mut rx);
    app.run(&mut rx, &mut evl);
}
