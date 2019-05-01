extern crate clap;
extern crate crypto;
extern crate threadpool;
use clap::{Arg, App, ArgGroup};
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
                let filename = p.canonicalize().unwrap();
                let bytes = read(&p).unwrap();
                let mut m = Md5::new();
                m.input(&bytes);
                let r = m.result_str();
                c.send(Hashed::Res(r, filename.as_path().to_str().unwrap().to_string())).unwrap();
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
                let e = a.entry(x);
                e.or_insert(vec!()).push(y);
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
        .group(ArgGroup::with_name("mode")
               .required(true))
        .arg(Arg::with_name("plan")
             .short("p")
             .long("plan")
             .group("mode")
             .help("Prints a mapping of every duplicate to the first instance."))
        .arg(Arg::with_name("replicas")
             .short("r")
             .long("replicas")
             .group("mode")
             .help("Prints files that duplicate the first instance."))
        .arg(Arg::with_name("duplicates")
             .short("d")
             .long("duplicates")
             .group("mode")
             .help("Prints files that occur more than once."))
        .arg(Arg::with_name("uniques")
             .short("u")
             .long("uniques")
             .group("mode")
             .help("Prints files that only occur once."))
        .arg(Arg::with_name("firsts")
             .short("f")
             .long("firsts")
             .group("mode")
             .help("Prints the first occurance of every unique file"))
        .arg(Arg::with_name("directory")
             .multiple(true)
             .help("Directories to search")
             .required(true)
             .takes_value(true))
        .get_matches();
    let pool = ThreadPool::new(12);
    let (tx, rx) = channel();


    if let Some(directories)  = matches.values_of("directory") {
        let directories = directories.into_iter().collect::<Vec<&str>>();

        let directory_hashes: HashMap<&str, HashMap<String, Vec<String>>> = directories.iter().map(|d| {
            let cnt = walk(&Path::new(&d), &pool, tx.clone()).unwrap();
            println!("reading {} files from {}", cnt, d);
            let h = reduce_hash_map(&rx, usize::from(cnt));
            (*d, h)
        }
        )
            .collect();

        let big_hash = directories.iter().map(|d| directory_hashes.get(d).unwrap() ).fold(HashMap::new(), |mut big, current| {
            for (k, mut v) in current {
                big.entry(k).or_insert(vec!()).extend(v);
            }
            big
        });

        for (h, v) in big_hash.iter() {
            let h = &h.to_string()[0..8];
            if matches.is_present("uniques") && v.len() == 1 {
                println!("({}) {}", h, v.first().unwrap());
            }

            if matches.is_present("duplicates") && v.len() > 1 {
                for duplicate in v {
                    println!("({}) {}", h, duplicate);
                }
            }

            if matches.is_present("firsts") {
                let f = v.first().unwrap();
                println!("({}) {}", h, f);
            }

            if matches.is_present("replicas") && v.len() > 1 {
                for duplicate in v.iter().skip(1) {
                    println!("({}) {}", h, duplicate);
                }
            }

            if matches.is_present("plan") && v.len() > 1 {
                let f = v.first().unwrap();
                for duplicate in v.iter().skip(1) {
                    println!("({:3}) {} -> {}", h, duplicate, f);
                }
            }
        }
    }
}
