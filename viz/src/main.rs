extern crate runic;
extern crate winit;

use std::io;
use std::io::{BufRead, BufReader};
use std::collections::HashMap;
use std::fs::File;

use runic::*;
use winit::*;

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

struct VizApp {
    perf_data: Vec<CallRecord>,
    method_index: HashMap<u32, String>,
    abs_start_time: u64,
    pixels_per_nanosecond: f32,
    offset_x: f32,
    font: Font
}

impl VizApp {
    fn init(rx: &mut RenderContext) -> VizApp {
        let mut args = std::env::args().skip(1);
        let data_f = BufReader::new(File::open(args.next().expect("perf data path")).expect("open perf data"));
        let data = read_from_psv(data_f).expect("load perf data");
        let mthd_f = BufReader::new(File::open(args.next().expect("method index path")).expect("open method index"));
        let mthd = read_method_index(mthd_f).expect("load method index");
        let start_time = data.iter().map(|&cr| cr.start_time).min().expect("minimum start time");
        VizApp {
            perf_data: data, method_index: mthd,
            abs_start_time: start_time, pixels_per_nanosecond: 0.00005, offset_x: -800.0,
            font: rx.new_font("Arial", 16.0, FontWeight::Regular, FontStyle::Normal).expect("load font")
        }
    }
}

impl App for VizApp {
    fn paint(&mut self, rx: &mut RenderContext) {
        rx.clear(Color::rgb(0.2, 0.2, 0.2));
        for cr in self.perf_data.iter()/*.take(2000)*/ {
            rx.set_color(Color::rgb(0.8, 0.6, (cr.method_id as f32 * 8.23).sin().abs()));
            let x = self.offset_x + (cr.start_time-self.abs_start_time) as f32 * self.pixels_per_nanosecond;
            let w = cr.elapsed_time as f32 * self.pixels_per_nanosecond;
            let r = Rect::xywh(x, 34.0*cr.depth as f32, w, 32.0);
            rx.fill_rect(r);
            rx.set_color(Color::rgb(0.4, 0.6, (cr.method_id as f32 * 8.23).sin().abs()));
            rx.stroke_rect(r, 2.0);
            if w > 128.0 {
                rx.set_color(Color::rgb(0.0, 0.0, 0.0));
                match self.method_index.get(&cr.method_id) {
                    Some(m) => rx.draw_text(r.offset(Point::xy(2.0, 2.0)), m, &self.font),
                    None => {}
                }
            }
        }
    }

    fn event(&mut self, e: Event) -> bool {
        match e {
            Event::WindowEvent { event: WindowEvent::KeyboardInput { input: k, .. }, .. } => {
                match k.virtual_keycode {
                    Some(VirtualKeyCode::Left) => {
                        self.offset_x -= 60.0;
                    },
                    Some(VirtualKeyCode::Right) => {
                        self.offset_x += 60.0;
                    }
                    Some(VirtualKeyCode::Up) => {
                        self.pixels_per_nanosecond -= 0.000001;
                    },
                    Some(VirtualKeyCode::Down) => {
                        self.pixels_per_nanosecond += 0.000001;
                    }
                    _ => {}
                }
            },
            _ => {}
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
