use std::io;
use std::io::{BufRead, BufReader};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::iter::FromIterator;
use std::path::{Path,PathBuf};

use std::error::Error;

use zip::read::*;

#[derive(Debug, Copy, Clone)]
/// Times in nanoseconds
pub struct CallRecord {
    pub thread_id: u32,
    pub start_time: u64,
    pub elapsed_time: u64,
    pub method_id: u32,
    pub depth: u32
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

pub struct VizData {
    pub calls: Vec<CallRecord>,
    pub method_index: HashMap<u32, String>,
    pub thread_ids: Vec<u32>,
    pub abs_end_time: u64,
    pub path: Option<PathBuf>,
    pub loaded: bool
}

fn flatten_opt_res<T,E,F: FnOnce()->E>(v: Option<Result<T,E>>, none_err: F) -> Result<T, E> {
    match v {
        Some(r) => r,
        None => Err(none_err())
    }
}

impl Default for VizData {
    fn default() -> VizData {
        VizData {
            calls: Vec::new(),
            method_index: HashMap::new(),
            thread_ids: Vec::new(),
            abs_end_time: 0,
            path: None,
            loaded: true
        }
    }
}

impl VizData {
    /// Create a new VizData from a data file and method index. Does not actually load data
    pub fn new<P: AsRef<Path>>(data_path: P) -> VizData {
        let mut dp = PathBuf::new(); dp.push(data_path);
        VizData {
            calls: Vec::new(),
            method_index: HashMap::new(),
            thread_ids: Vec::new(),
            abs_end_time: 0,
            path: Some(dp),
            loaded: false
        }
    }

    /// Load data from files if it is unloaded
    pub fn load(data: ::std::sync::Arc<::std::sync::RwLock<VizData>>) -> Result<(), io::Error> {
        let mut ach = {
            let s = data.read().unwrap();
            ZipArchive::new(File::open(&s.path.as_ref().expect("data not associated with path"))?)?
        };
        {
            let mut header_f = (BufReader::new(ach.by_name("header")?)).lines();
            let mut vd = data.write().unwrap();
            vd.loaded = false;
            vd.calls.clear(); vd.method_index.clear();
            vd.thread_ids = Vec::new();
            for id in flatten_opt_res(header_f.next(), ||io::Error::new(io::ErrorKind::UnexpectedEof, ""))?.split(';').filter(|x| x.len()!=0).map(|v| v.parse::<u32>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))) {
                    
                        vd.thread_ids.push(id?);
            }
            vd.abs_end_time = flatten_opt_res(header_f.next(), ||io::Error::new(io::ErrorKind::UnexpectedEof, ""))?.parse::<u64>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        }
        {(data.write().unwrap().method_index) = read_method_index(BufReader::new(ach.by_name("methods")?))?; }
        let mut res = Vec::new();
        {
            let data_f = BufReader::new(ach.by_name("data")?);
            for linep in data_f.lines().skip(1) {
                let cr = CallRecord::from_psv(&linep?)?;
                res.push(cr);
                if res.len() > 16 {
                    data.write().unwrap().calls.append(&mut res);
                }
            }
            data.write().unwrap().calls.append(&mut res);
        }
        data.write().unwrap().loaded = true;
        Ok(())
    }
}

