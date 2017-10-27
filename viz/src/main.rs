#![feature(conservative_impl_trait)]
extern crate runic;
extern crate winit;
extern crate futures;
extern crate futures_cpupool;

use std::io;
use std::io::{BufRead, BufReader};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::iter::FromIterator;

use runic::*;
use winit::*;

use std::thread;
use std::thread::Thread;
use std::sync::{Arc, RwLock, TryLockError};

#[derive(Debug, Copy, Clone)]
/// Times in nanoseconds
struct CallRecord {
    thread_id: u32,
    start_time: u64,
    elapsed_time: u64,
    method_id: u32,
    depth: u32
}

impl CallRecord {
    fn from_psv(s: &String) -> Result<CallRecord, io::Error> {
        let mut items = s.split('|');
        Ok(CallRecord {
            thread_id: items.next().ok_or(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
                .and_then(|v| v.parse::<u32>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)))?,
            start_time: items.next().ok_or(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
                .and_then(|v| v.parse::<u64>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)))?,
            elapsed_time: items.next().ok_or(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
                .and_then(|v| v.parse::<u64>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)))?,
            method_id: items.next().ok_or(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
                .and_then(|v| v.parse::<u32>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)))?,
            depth: items.next().ok_or(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
                .and_then(|v| v.parse::<u32>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)))?,
        })
    }
}

fn read_from_psv<R: BufRead>(data: R) -> Result<Vec<CallRecord>, io::Error> {
    let mut res = Vec::new();
    for linep in data.lines().skip(1) {
        match linep {
            Ok(l) => res.push(CallRecord::from_psv(&l)?),
            Err(e) => return Err(e)
        }
    }
    Ok(res)
}

fn read_method_index<R: BufRead>(data: R) -> Result<HashMap<u32, String>, io::Error> {
    let mut ix = HashMap::new();
    for linep in data.lines().skip(1) {
        match linep {
            Ok(line) =>  {
                let mut items = line.split('|');
                ix.insert(items.next().ok_or(io::Error::new(io::ErrorKind::UnexpectedEof, ""))
                    .and_then(|v| v.parse::<u32>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)))?,
                    String::from(items.next().ok_or(io::Error::new(io::ErrorKind::UnexpectedEof, ""))?));
            },
            Err(e) => return Err(e)
        }
    }
    Ok(ix)
}

struct VizData {
    perf_data: Vec<CallRecord>,
    method_index: HashMap<u32, String>,
    thread_ids: Vec<u32>,
    abs_end_time: u64,
}

impl VizData {
    fn load<P: AsRef<::std::path::Path>>(perf_path: P, method_path: P) -> Result<VizData, io::Error> {
        let data_f = BufReader::new(File::open(perf_path)?);//.expect("open perf data"));
        let mthd_f = BufReader::new(File::open(method_path)?);//.expect("open method index"));
        let data = match read_from_psv(data_f) {
            Ok(v) => v,
            Err(e) => { return Err(e); }
        };
        let mthd = match read_method_index(mthd_f) {
            Ok(v) => v,
            Err(e) => { return Err(e); }
        };
        let mut start_time: u64 = ::std::u64::MAX;
        let mut end_time: u64 = 0;
        let mut tids = HashSet::new();
        for cr in data.iter() {
            if cr.start_time < start_time { start_time = cr.start_time; }
            if cr.start_time+cr.elapsed_time > end_time { end_time = cr.start_time+cr.elapsed_time; }
            tids.insert(cr.thread_id);
        }
        Ok(VizData {
            perf_data: data, method_index: mthd,
            thread_ids: Vec::from_iter(tids.iter().map(|&v| v)),
            //abs_start_time: start_time,
            abs_end_time: end_time,
        })
    }

    fn paint(&self, rx: &mut RenderContext, current_thread_ix: usize, offset_x: i64, pixels_per_nanosecond: f32, last_mouse: Point, font: &Font) {
        let bounds = rx.bounds();
        let current_thread_ix = if current_thread_ix == 0 { 0 } else { self.thread_ids[current_thread_ix-1] };

        let status_tx = rx.new_text_layout(&format!("@{}, {:.2}% | Thread #{} | {} records", -offset_x, ((-offset_x) as f64 / (self.abs_end_time) as f64)*100.0, current_thread_ix, self.perf_data.len()), &font, bounds.w, bounds.h).expect("create status text layout");
        rx.set_color(Color::rgb(0.3, 0.3, 0.3));
        rx.fill_rect(Rect::xywh(0.0, 0.0, bounds.w, status_tx.bounds().h+2.0));
        rx.set_color(Color::rgb(0.8, 0.8, 0.8));
        rx.draw_text_layout(Point::xy(2.0, 0.0), &status_tx);

        let mut hovered_record: Option<&CallRecord> = None;
        for cr in self.perf_data.iter() {
            if current_thread_ix > 0 && cr.thread_id != current_thread_ix { continue; }
            let w = cr.elapsed_time as f32 * pixels_per_nanosecond;
            if w < 2.0 { continue; }
            rx.set_color(Color::rgb(0.8, 0.6, (cr.method_id as f32 * 8.23).sin().abs()));
            let x = (offset_x + (cr.start_time) as i64) as f32 * pixels_per_nanosecond;
            if x+w < 0.0 || x > bounds.w { continue; }
            let r = Rect::xywh(x, 34.0*cr.depth as f32, w, 32.0);
            rx.fill_rect(r);
            if r.contains(last_mouse) {
                hovered_record = Some(cr);
                rx.set_color(Color::rgb(0.6, 0.2, (cr.method_id as f32 * 8.23).sin().abs()));
            } else {
                rx.set_color(Color::rgb(0.2, 0.4, (cr.method_id as f32 * 8.23).sin().abs()));
            }
            rx.stroke_rect(r, 2.0);
            if w > 128.0 {
                rx.set_color(Color::rgb(0.0, 0.0, 0.0));
                match self.method_index.get(&cr.method_id) {
                    Some(m) => {
                        let tr = Rect::xywh(r.x.max(0.0) + 2.0, r.y + 2.0, r.w, r.h);
                        rx.draw_text(tr, m, &font)
                    },
                    None => {}
                }
            }
        }
        // draw tooltip
        if let Some(cr) = hovered_record {
            let tx = rx.new_text_layout(&format!("{}\nStart Time: {}ns\nElapsed Time: {}ns\nThread #{}, Depth {}", self.method_index.get(&cr.method_id).unwrap_or(&String::from("?")), cr.start_time, cr.elapsed_time, cr.thread_id, cr.depth), &font, bounds.w, bounds.h).expect("create tooltip layout");
            rx.set_color(Color::rgb(0.3, 0.3, 0.3));
            let mut ttb = tx.bounds().offset(last_mouse).offset(Point::xy(16.0, 0.0));//Rect::pnwh(self.last_mouse, 128.0, 128.0);
            ttb.w += 8.0; ttb.h += 8.0;
            rx.fill_rect(ttb);
            rx.set_color(Color::rgb(0.8, 0.8, 0.8));
            rx.draw_text_layout(Point::xy(4.0 + ttb.x, 4.0 + ttb.y), &tx);
            rx.set_color(Color::rgb(0.6, 0.6, 0.6));
            rx.stroke_rect(ttb, 2.0);
        }
    }
}

struct VizApp {
    data: Arc<RwLock<VizData>>,
    current_thread_id: usize, max_thread_ix: usize,
    pixels_per_nanosecond: f32,
    offset_x: i64,
    font: Font,
    last_mouse: Point,
    bounds: Rect
}

impl VizApp {
    fn init(rx: &mut RenderContext) -> VizApp {
        /*let data = futures::executor::spawn(pool.spawn_fn(|| {
            println!("hello from worker thread!");
    
        }));*/
        let mut args = std::env::args().skip(1);
        let (perf_path, method_path) = (args.next().expect("perf data path"), args.next().expect("method index path"));
        let mut data = Arc::new(RwLock::new(VizData {
            perf_data: Vec::new(),
            method_index: HashMap::new(),
            thread_ids: Vec::new(),
            abs_end_time: 10000
        }));
        let mut tdata = data.clone();
        let data_f = BufReader::new(File::open(perf_path).expect("open perf data"));
        let mthd_f = BufReader::new(File::open(method_path).expect("open method index"));
        let load_thread = thread::spawn(move || {
            /*let data = match read_from_psv(data_f) {
                Ok(v) => v,
                Err(e) => { return Err(e); }
            };*/
            let mthd = read_method_index(mthd_f).expect("read method index");
            { (data.write().unwrap()).method_index = mthd; }
            let mut res = Vec::new();
            let mut start_time: u64 = ::std::u64::MAX;
            let mut end_time: u64 = 0;
            let mut tids = HashSet::new();
            for linep in data_f.lines().skip(1) {
                let cr = CallRecord::from_psv(&linep.expect("read line")).expect("parse record");
                if cr.start_time < start_time { start_time = cr.start_time; }
                if cr.start_time+cr.elapsed_time > end_time { end_time = cr.start_time+cr.elapsed_time; }
                tids.insert(cr.thread_id);
                res.push(cr);
                if res.len() > 16 {
                    data.write().unwrap().perf_data.append(&mut res);
                }
            }
            {
                let mut vd = data.write().unwrap();
                vd.thread_ids = Vec::from_iter(tids.iter().map(|&v| v));
                vd.abs_end_time = end_time;
            }
            //*(data.write().unwrap()) = VizData::load(paths.0, paths.1).map_err(|e|  {println!("error {}", e); e }).ok();
        });
        //
        VizApp {
            data: tdata,
            pixels_per_nanosecond: 0.00005, offset_x: 0, current_thread_id: 0, max_thread_ix: 0,
            font: rx.new_font("Arial", 16.0, FontWeight::Regular, FontStyle::Normal).expect("load font"), last_mouse: Point::xy(0.0, 0.0),
            bounds:rx.bounds()
        }
    }
}

impl App for VizApp {
    fn paint(&mut self, rx: &mut RenderContext) {
        self.bounds = rx.bounds();
        rx.clear(Color::rgb(0.2, 0.2, 0.2));

        match self.data.try_read() {
            Ok(d) => {
                self.max_thread_ix = d.thread_ids.len();
                d.paint(rx, self.current_thread_id, self.offset_x, self.pixels_per_nanosecond, self.last_mouse, &self.font)
            },
            Err(TryLockError::WouldBlock) => {
                rx.set_color(Color::rgb(0.7, 0.7, 0.7));
                rx.draw_text(Rect::xywh(32.0, 32.0, self.bounds.w, self.bounds.h), "loading...", &self.font);
            },
            Err(TryLockError::Poisoned(e)) => {
                panic!("poisoned lock {}", e);
            }
        } 
    }

    fn event(&mut self, ev: Event) -> bool {
        if let Event::WindowEvent { event: e, .. } = ev {
            match e {
                WindowEvent::KeyboardInput { input: k, .. } => {
                    match k.virtual_keycode {
                        Some(VirtualKeyCode::Left) => {
                            self.offset_x += ((self.bounds.w * 0.1) / self.pixels_per_nanosecond) as i64; //600000;
                        },
                        Some(VirtualKeyCode::Right) => {
                            self.offset_x -= ((self.bounds.w * 0.1) / self.pixels_per_nanosecond) as i64; //600000;
                        }
                        Some(VirtualKeyCode::Up) => {
                            self.pixels_per_nanosecond -= 0.000001;
                        },
                        Some(VirtualKeyCode::Down) => {
                            self.pixels_per_nanosecond += 0.000001;
                        },
                        Some(VirtualKeyCode::PageUp) => {
                            if k.state == ElementState::Released {
                                
                                if self.current_thread_id < self.max_thread_ix {
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
                WindowEvent::MouseMoved { position: (x,y), .. } => {
                    self.last_mouse = Point::xy(x as f32, y as f32);
                }
                _ => {}
            }
        }
        false
    }
}

fn main() {
    runic::init();
    let mut evl = EventsLoop::new();
    let mut window = WindowBuilder::new().with_dimensions(512, 521).with_title("Java Performance Visualizer").build(&evl).expect("create window!");
    let mut rx = RenderContext::new(&mut window).expect("create render context!");
    let mut app = VizApp::init(&mut rx);
    app.run(&mut rx, &mut evl);
}
