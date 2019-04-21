extern crate clap;
use clap::{Arg, App, SubCommand};
use std::fs::{read_dir};
use std::path::Path;


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
        for entry in read_dir(&path).unwrap() {
            println!("filse : {}", entry.unwrap().path().to_str().unwrap());
        }
}


