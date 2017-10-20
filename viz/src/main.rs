extern crate runic;
extern crate winit;

use std::io;
use std::io::{BufRead, BufReader};

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

struct VizApp {
    perf_data: Vec<CallRecord>,
    abs_start_time: u64,
    pixels_per_nanosecond: f32
}

impl VizApp {
    fn init() -> VizApp {
        let f = File::open("..\\perfdata.csv").expect("open perf data");
        let f = BufReader::new(f);
        let data = read_from_psv(f).expect("load perf data");
        let start_time = data.iter().map(|&cr| cr.start_time).min().expect("minimum start time");
        VizApp { perf_data: data, abs_start_time: start_time, pixels_per_nanosecond: 0.00001 }
    }
}

impl App for VizApp {
    fn paint(&mut self, rx: &mut RenderContext) {
        rx.clear(Color::rgb(0.2, 0.2, 0.2));
        for cr in self.perf_data.iter()/*.take(2000)*/ {
            rx.set_color(Color::rgb(0.8, 0.6, (cr.method_id as f32 * 1.23).sin().abs()));
            let x = (cr.start_time-self.abs_start_time) as f32 * self.pixels_per_nanosecond;
            let w = cr.elapsed_time as f32 * self.pixels_per_nanosecond;
            rx.fill_rect(Rect::xywh(x, 34.0*cr.depth as f32, w, 32.0));
        }
    }

    fn event(&mut self, _: Event) -> bool {
        false
    }
}

fn main() {
    runic::init();
    let mut evl = EventsLoop::new();
    let mut window = WindowBuilder::new().with_dimensions(512, 521).with_title("Basic Window").build(&evl).expect("create window!");
    let mut rx = RenderContext::new(&mut window).expect("create render context!");
    let mut app = VizApp::init();
    app.run(&mut rx, &mut evl);
}
