extern crate clap;
extern crate crypto;
extern crate threadpool;
use clap::{Arg, App};
use std::fs::{read_dir, read};
use std::path::Path;
use crypto::md5::Md5;
use crypto::digest::Digest;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Sender, Receiver};
use threadpool::ThreadPool;

enum Hashed {
    Res (String, String)
}


fn walk(path: &Path, pool: &ThreadPool, tx:  Sender<Hashed>) -> Result<usize, std::io::Error> {
    let mut cnt = 0;
    for entry in read_dir(&path)? {
        let p = entry.unwrap().path();
        if p.is_file() {
            cnt += 1;
            let c = tx.clone();
            pool.execute(move || {
                let filename = p.to_str().unwrap();
                let bytes = read(&p).unwrap();
                let mut m = Md5::new();
                m.input(&bytes);
                let r = m.result_str();
                c.send(Hashed::Res(r, filename.to_string())).unwrap();
            });
        } else if p.is_dir() {
            cnt += walk(&p, pool, tx.clone()).unwrap();
        }
    }

    return Ok(cnt);
}

fn reduce_hash_map(rx: &Receiver<Hashed>, cnt: usize) -> HashMap<String, Vec<String>> {
    rx.iter().take(cnt).fold(HashMap::new(), |mut a, z| {
        match z {
            Hashed::Res(x, y) => {
                a.entry(x)
                    .and_modify(|result| result.push(y.clone()))
                    .or_insert(vec!(y.clone()));
                a
            }
        }
    })
}

fn main() {
    let matches = App::new("filesets")
        .version("1.0")
        .author("Bryce Covert")
        .about("compares two directory trees, telling you which files are in one but not in the other")
        .arg(Arg::with_name("print")
             .short("p")
             .long("print")
             .takes_value(true)
             .multiple(true)
             .required(true)
             .help("What to print. Valid options: uniques, duplicates, rolling-uniques, rolling-duplicates"))
        .arg(Arg::with_name("directory")
             .short("d")
             .long("directory")
             .multiple(true)
             .help("A directory to build sets for")
             .required(true)
             .takes_value(true))
        .get_matches();
    let pool = ThreadPool::new(12);
    let (tx, rx) = channel();

    let prints = matches.values_of("print").unwrap().into_iter().map(|p| p.to_string()).collect::<HashSet<String>>();

    if let Some(directories)  = matches.values_of("directory") {
        let directories = directories.into_iter().map(|x| x.to_string()).collect::<Vec<String>>();
        let directories2 = directories.into_iter().collect::<Vec<String>>();
        let directory_hashes: HashMap<String, HashMap<String, Vec<String>>> = directories.iter().map(|d| {
            let cnt = walk(&Path::new(&d), &pool, tx.clone()).unwrap();
            let h = reduce_hash_map(&rx, usize::from(cnt));
            println!("{} has {} files", d, cnt);
            (d.clone(), h)
        }
        )
            .collect::<HashMap<String, HashMap<String, Vec<String>>>>();
        let mut locations = HashMap::<&String, &String>::new(); 
        let mut seen = HashSet::<&String>::new(); 
        for directory in directories.iter() {
            let others = directories.iter().filter(|d| *d != directory)
                .flat_map(|d| directory_hashes.get(d).unwrap().keys())
                .collect::<HashSet<&String>>();

            let mine_map = directory_hashes.get(directory).unwrap();
            let mine = mine_map.keys().collect::<HashSet<&String>>();

            if prints.contains("uniques") {
                for result in mine.difference(&others).flat_map(|h| directory_hashes.get(directory).unwrap().get(h.clone()).unwrap()) {
                    println!("{} is unique", result);
                }
            }

            if prints.contains("duplicates") {
                for result in mine.intersection(&others) {
                    for duplicate in mine_map.get(&result.to_string()).unwrap() {
                        if let Some(l) = locations.get(result) {
                            println!("{} -> {}", duplicate, l);
                        }
                    }
                }
            }

            if prints.contains("rolling-duplicates") {
                for result in mine.intersection(&seen) {
                    for duplicate in mine_map.get(&result.to_string()).unwrap() {
                        if let Some(l) = locations.get(result) {
                            println!("{} -> {}", duplicate, l);
                        }
                    }
                }
            }

            if prints.contains("rolling-uniques") {
                for result in mine.difference(&seen) {
                    
                    for unique in mine_map.get(&result.to_string()).unwrap() {
                        println!("{} is rolling-unique", unique);
                    }
                }
            }

            for file in mine.iter() {
                let filename = directory_hashes.get(directory).unwrap().get(file.clone()).unwrap().get(0).unwrap();
                locations.entry(file).or_insert(filename);
                seen.insert(file);
            }
        }
    }
}
