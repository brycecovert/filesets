extern crate clap;
extern crate crypto;
use clap::{Arg, App, SubCommand};
use std::fs::{read_dir, read};
use std::path::Path;
use crypto::md5::Md5;
use crypto::digest::Digest;


fn printMd5(path: &Path) -> i32{
    let mut x = 0;
    for entry in read_dir(&path).unwrap() {
        let p = entry.unwrap().path();
        if (p.is_file()) {
            let filename = p.to_str().unwrap();
            let bytes = read(&p).unwrap();
            let mut m = Md5::new();
            m.input(&bytes);
            x = x + 1;
        } else if (p.is_dir()) {
            x = x + printMd5(&p);
        }
    }
    return x
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
        .get_matches();
        let path = Path::new(matches.value_of("left").unwrap());
    let cnt = printMd5(&path);
    println!("{}", cnt);
}


