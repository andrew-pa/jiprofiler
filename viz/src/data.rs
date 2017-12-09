use std::io;
use std::io::{BufRead, BufReader};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::iter::FromIterator;
use std::path::{Path,PathBuf};

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
    path: (PathBuf, PathBuf)
}

impl VizData {
    /// Create a new VizData from a data file and method index. Does not actually load data
    pub fn new<P: AsRef<Path>>(data_path: P, method_index_path: P) -> Result<VizData, io::Error> {
        let mut dp = PathBuf::new(); dp.push(data_path);
        let mut mp = PathBuf::new(); mp.push(method_index_path);
        Ok(VizData {
            calls: Vec::new(),
            method_index: HashMap::new(),
            thread_ids: Vec::new(),
            abs_end_time: 0,
            path: (dp, mp)
        })
    }

    /// Load data from files if it is unloaded
    pub fn load(data: ::std::sync::Arc<::std::sync::RwLock<VizData>>) -> Result<(), io::Error> {
        let (data_f, mthd_f) = {
            let s = data.read().unwrap();
            (BufReader::new(File::open(&s.path.0)?),
             BufReader::new(File::open(&s.path.1)?))
        };
        { (data.write().unwrap().method_index) = read_method_index(mthd_f)?; }
        let mut res = Vec::new();
        let mut start_time: u64 = ::std::u64::MAX;
        let mut end_time: u64 = 0;
        let mut tids = HashSet::new();
        for linep in data_f.lines().skip(1) {
            let cr = CallRecord::from_psv(&linep?)?;
            if cr.start_time < start_time { start_time = cr.start_time; }
            if cr.start_time+cr.elapsed_time > end_time { end_time = cr.start_time+cr.elapsed_time; }
            tids.insert(cr.thread_id);
            res.push(cr);
            if res.len() > 16 {
                data.write().unwrap().calls.append(&mut res);
            }
        }
        data.write().unwrap().calls.append(&mut res);
        {
            let mut vd = data.write().unwrap();
            vd.thread_ids = Vec::from_iter(tids.iter().map(|&v| v));
            vd.abs_end_time = end_time;
        }
        Ok(())
    }
}

