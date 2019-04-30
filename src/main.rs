extern crate clap;
extern crate crypto;
extern crate threadpool;
use clap::{Arg, App, SubCommand};
use std::fs::{read_dir, read};
use std::path::Path;
use crypto::md5::Md5;
use crypto::digest::Digest;
use std::collections::{HashMap, HashSet};
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

fn reduceHashmap(rx: &Receiver<Hashed>, cnt: usize) -> HashMap<String, Vec<String>> {
    rx.iter().take(cnt).fold(HashMap::new(), |mut a, z| {
        match z {
            Hashed::Res(x, y) => {
                a.entry(x)
                    .and_modify(|result| result.push(y.clone()))
                    .or_insert(vec!(y.clone()));
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
        .arg(Arg::with_name("directory")
             .short("d")
             .long("directory")
             .multiple(true)
             .help("A directory to build sets for")
             .required(true)
             .takes_value(true))
        .get_matches();
    let pool = ThreadPool::new(12);
    let (mut tx, mut rx) = channel();



    if let Some(directories)  = matches.values_of("directory") {
        let hashMaps: Vec<HashMap<String, Vec<String>>> = directories.into_iter().map(|d| {
            let cnt = walk(&Path::new(d), &pool, tx.clone()).unwrap();
            let h = reduceHashmap(&rx, usize::from(cnt));
            println!("{} has {} files ({} unique)", d, cnt, h.len() );
            h
        })
            .collect();

        let mut seen = hashMaps.first().unwrap().keys().collect::<HashSet<&String>>();
        let mut locations = hashMaps.first().unwrap().iter().map(|(k, v)| (k, v.first().unwrap())).collect::<HashMap<&String,&String>>();
        for directoryMap in hashMaps.iter() {
            for alreadySeen in directoryMap.keys() {
                if (seen.contains(alreadySeen)) {
                    for seenInstance in directoryMap.get(alreadySeen).unwrap() {
                        println!("{} -> {}", seenInstance, locations.get(alreadySeen).unwrap());
                    }
                } else {
                    seen.insert(&alreadySeen);
                    locations.insert(&alreadySeen, directoryMap.get(alreadySeen).unwrap().first().unwrap());
                }
            }
        }

    }
}
