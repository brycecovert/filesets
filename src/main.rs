extern crate clap;
extern crate crypto;
extern crate threadpool;
extern crate pbr;
use clap::{Arg, App, ArgGroup};
use std::fs::{read};
use std::path::Path;
use crypto::md5::Md5;
use crypto::digest::Digest;
use std::io::{stdout, Write};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Sender};
use threadpool::ThreadPool;
use walkdir::WalkDir;
use pbr::ProgressBar;

enum Hashed {
    Res (String, String),
    Err
}

fn walk(path: &Path, pool: &ThreadPool, tx:  Sender<Hashed>, quiet: bool) -> Result<u64, std::io::Error> {
    let mut cnt = 0;
    for entry in WalkDir::new(&path)
        .follow_links(false)
        .into_iter()
        .map(|x| x.unwrap()){
            if !entry.metadata().unwrap().is_dir()  {
                cnt += 1;
                if !quiet && cnt % 100 == 0 {
                    print!("\rScanning {} ({} found)\0", path.to_str().unwrap(), cnt);
                    stdout().flush().ok().expect("Could not flush stdout");
                }
                let c = tx.clone();
                pool.execute(move || {
                    if let Ok(bytes) = read(&entry.path()) {
                        let mut m = Md5::new();
                        m.input(&bytes);
                        let r = m.result_str();
                        if let Some(path2) = entry.path().to_str() {
                            c.send(Hashed::Res(r, path2.to_string())).unwrap();
                        } else {
                            c.send(Hashed::Err).unwrap();
                        }
                    } else {
                        c.send(Hashed::Err).unwrap();
                    }
                });
            }
        }
    return Ok(cnt);
}


fn build_fileset(directories: &Vec<&str>, quiet: bool) -> HashMap<String, Vec<String>> {
    let pool = ThreadPool::new(12);

    let mut seen = HashSet::<String>::new();
    let (tx, rx) = channel();
    directories.iter().flat_map(|d| {
        let cnt = walk(&Path::new(&d), &pool, tx.clone(), quiet).unwrap();
        if quiet {
            rx.iter()
                .take (cnt as usize)
                .collect::<Vec<_>>()
        } else {
            let mut pb = ProgressBar::new(u64::from(cnt));
            pb.show_speed =false;
            pb.format("[=> ]");
            let z = rx.iter()
                .take (cnt as usize)
                .map(|d| { pb.inc(); d})
                .collect::<Vec<_>>();
            pb.finish();
            z
        }
    })
    .fold(HashMap::new(), |mut big, current| {
        match current {
            Hashed::Res(k, v) => {
                let entry = big.entry(k.to_string()).or_insert(vec!());
                let canonical = Path::new(&v).canonicalize().unwrap().as_os_str().to_str().unwrap().to_owned();

                if !seen.contains(&canonical) {
                    entry.push(v.clone());
                }
                seen.insert(canonical);
            }
            _ => ()
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
