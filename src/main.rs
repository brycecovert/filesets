extern crate clap;
extern crate crypto;
extern crate threadpool;
use clap::{Arg, App, SubCommand};
use std::fs::{read_dir, read};
use std::path::Path;
use crypto::md5::Md5;
use crypto::digest::Digest;
use std::collections::HashMap;
use std::thread::Builder;
use std::sync::mpsc::{channel, Sender, Receiver};
use threadpool::ThreadPool;

enum Hashed {
    Res (String, String),
    Done
}


fn walk(path: &Path, pool: &ThreadPool, tx:  Sender<Hashed>) -> Result<usize, std::io::Error> {
    let mut cnt = 0;
    for entry in read_dir(&path)? {
        let p = entry.unwrap().path();
        if (p.is_file()) {
            cnt += 1;
            let c = tx.clone();
            pool.execute(move || {
                let filename = p.to_str().unwrap();
                let bytes = read(&p).unwrap();
                let mut m = Md5::new();
                m.input(&bytes);
                let r = m.result_str();
                c.send(Hashed::Res(r, filename.to_string()));
            });
        } else if (p.is_dir()) {
            cnt += walk(&p, pool, tx.clone()).unwrap();
        }
    }

    return Ok(cnt);
}

fn reduceHashmap(rx: Receiver<Hashed>, cnt: usize) -> HashMap<String, String> {
    rx.iter().take(cnt).fold(HashMap::new(), |mut a, z| {
        match z {
            Hashed::Res(x, y) => {
                a.insert(x, y);
                a
            }
            _ => a
        }
    })
}


fn main() {
    let matches = App::new("filesets")
        .version("1.0")
        .author("Bryce Covert")
        .about("compares two directory trees, telling you which files are in one but not in the other")
        .arg(Arg::with_name("left")
             .short("l")
             .long("left")
             .value_name("LEFT")
             .help("The first directory")
             .required(true)
             .takes_value(true))
        .arg(Arg::with_name("right")
             .short("r")
             .long("right")
             .value_name("RIGHT")
             .help("The second directory")
             .required(true)
             .takes_value(true))
        .get_matches();
    let pool = ThreadPool::new(12);
    let (mut tx, mut rx) = channel();
    let cnt = walk(&Path::new(matches.value_of("left").unwrap()), &pool, tx.clone()).unwrap();
    let h = reduceHashmap(rx, usize::from(cnt));
    println!("found, {} in left", cnt );

    let pool2 = ThreadPool::new(12);
    let (mut tx, mut rx) = channel();
    let cnt = walk(&Path::new(matches.value_of("right").unwrap()), &pool, tx.clone()).unwrap();
    let h2 = reduceHashmap(rx, usize::from(cnt));
    println!("found, {} in right", cnt );
    for (key, val) in h2.iter() {
        if (!h.contains_key(key)) {
            println!("key: {}, value: {}", key, val);
         }
    }
}


