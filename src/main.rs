extern crate clap;
extern crate crypto;
extern crate threadpool;
extern crate pbr;
use clap::{Arg, App, ArgGroup};
use std::fs::{read_dir, read};
use std::path::Path;
use crypto::md5::Md5;
use crypto::digest::Digest;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Sender, Receiver};
use threadpool::ThreadPool;
use pbr::ProgressBar;
use std::io::Write;

enum Hashed {
    Res (String, String),
        Err
}

fn walk(path: &Path, pool: &ThreadPool, tx:  Sender<Hashed>) -> Result<u64, std::io::Error> {
    let mut cnt = 0;
    for entry in read_dir(&path)? {
        if let Ok(p) = entry {
            let p = p.path();
            if p.is_file() {
                cnt += 1;
                let c = tx.clone();
                pool.execute(move || {
                    let filename = p;
                    if let Ok(bytes) = read(&filename) {
                        let mut m = Md5::new();
                        m.input(&bytes);
                        let r = m.result_str();
                        if let Some(path2) = filename.to_str() {
                            c.send(Hashed::Res(r, path2.to_string())).unwrap();
                        } else {
                            c.send(Hashed::Err);
                        }
                    } else {
                        c.send(Hashed::Err);
                    }
                });
            } else if p.is_dir() {
                if let Ok(more) = walk(&p, pool, tx.clone()) {
                    cnt += more;
                }
            }
        }
    }

    return Ok(cnt);
}

fn reduce_hash_map<T: Write >(rx: &Receiver<Hashed>, cnt: u64, pb: &mut Option<ProgressBar<T>>) -> HashMap<String, Vec<String>> {
    rx.iter().take(cnt as usize).fold(HashMap::new(), |mut a, z| {
        match z {
            Hashed::Res(x, y) => {
                if let Some(pb2) = pb {
                    pb2.inc();
                };
                let e = a.entry(x);
                e.or_insert(vec!()).push(y);
                a
            },
            Hashed::Err => {
                if let Some(pb) = pb {
                    pb.inc();
                }
                    a
            }
        }
    })
}

fn build_fileset(directories: &Vec<&str>, quiet: bool) -> HashMap<String, Vec<String>> {
    let pool = ThreadPool::new(12);
    let (tx, rx) = channel();

    let directory_hashes: HashMap<&str, HashMap<String, Vec<String>>> = directories.iter().map(|d| {
        let cnt = walk(&Path::new(&d), &pool, tx.clone()).unwrap();
        let mut pb = match quiet
        {
            false => Some(ProgressBar::new(u64::from(cnt))).and_then(|mut pb| {
                pb.show_speed =false;
                pb.format("[=> ]");
                Some(pb)
            }),
            true => None
        };
        let h = reduce_hash_map(&rx, cnt, &mut pb);
        if let Some( pb2) = &mut pb {
            pb2.finish();
        }
        (*d, h)
    }
    )
        .collect();

    let mut seen = HashSet::<String>::new();
    directories.iter().map(|d| directory_hashes.get(d).unwrap() ).fold(HashMap::new(), |mut big, current| {
        for (k, mut v) in current {
            let entry = big.entry(k.to_string()).or_insert(vec!());
            for file in v {
                let canonical = Path::new(file).canonicalize().unwrap().as_os_str().to_str().unwrap().to_owned();

                if !seen.contains(&canonical) {
                    entry.push(file.clone());
                }
                seen.insert(canonical);
            }
        }
        big
    })
}

fn main() {
    let matches = App::new("filesets")
        .version("1.0")
        .author("Bryce Covert")
        .about("Your swiss-army knife for dealing with identical files.")

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
        .arg(Arg::with_name("quiet")
             .short("q")
             .long("quiet")
             .help("Doesn't print progress"))
        .arg(Arg::with_name("directory")
             .multiple(true)
             .help("Directories to search")
             .required(true)
             .takes_value(true))
        .get_matches();

    if let Some(directories)  = matches.values_of("directory") {
        let directories = directories.into_iter().collect::<Vec<&str>>();

        for (h, v) in build_fileset(&directories, matches.is_present("quiet")).iter() {
            let h = &h.to_string()[0..8];
            if matches.is_present("uniques") && v.len() == 1 {
                if !matches.is_present("quiet") {
                    print!("({}) ", h);
                }
                println!("{}", v.first().unwrap());
            }

            if matches.is_present("duplicates") && v.len() > 1 {
                for duplicate in v {
                    if !matches.is_present("quiet") {
                        print!("({}) ", h);
                    }
                    println!("{}", duplicate);
                }
            }

            if matches.is_present("firsts") {
                if !matches.is_present("quiet") {
                    print!("({}) ", h);
                }
                let f = v.first().unwrap();
                println!("{}", f);
            }

            if matches.is_present("replicas") && v.len() > 1 {
                for duplicate in v.iter().skip(1) {
                    if !matches.is_present("quiet") {
                        print!("({}) ", h);
                    }
                    println!("{}", duplicate);
                }
            }

            if matches.is_present("plan") && v.len() > 1 {
                let f = v.first().unwrap();
                for duplicate in v.iter().skip(1) {
                    if !matches.is_present("quiet") {
                        print!("({}) ", h);
                    }
                    println!("{} -> {}", duplicate, f);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::build_fileset;
    use std::collections::HashMap;

    #[test]
    fn it_should_find_individual_files() {
        let expected: HashMap<String, Vec<String>> = vec!(
            ("8d4dd6ee05b14bed218bdd8e4e89f648".to_string(),
             vec!("examples/1/unique-1".to_string())),
            ("0bc0878606ed744ae45696e6faad0c03".to_string(),
             vec!("examples/1/duplicated2".to_string())))
            .into_iter().collect();
        assert_eq!(build_fileset(&vec!["examples/1"], true), expected);
    }

    #[test]
    fn it_should_find_duplicate_files() {
        assert_eq!(build_fileset(&vec!["examples/1", "examples/3"], true)
                   .get("0bc0878606ed744ae45696e6faad0c03").unwrap(),
                   &vec!("examples/1/duplicated2".to_string(), "examples/3/duplicated2".to_string()));
    }

    #[test]
    fn it_should_dedup_same_relative_file() {
        assert_eq!(build_fileset(&vec!["examples/1", "examples/../examples/1"], true)
                   .get("0bc0878606ed744ae45696e6faad0c03").unwrap(),
                   &vec!("examples/1/duplicated2".to_string()));
    }
}
