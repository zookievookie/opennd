use std::path::*;
use std::thread;
use clap::Parser;
use std::time::{Instant, Duration};

mod avf;
mod encodepng;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    pub procedure: String,
    #[arg(short, long, default_value = "/")]
    pub input: PathBuf,
    #[arg(short, long)]
    pub output: PathBuf,
    #[arg(short, long, default_value = "NONE")]
    pub ref_avf: PathBuf,
}

fn main() {
    let timecheck: Instant = Instant::now();
    let args = Cli::parse();
    match args.procedure.as_str() {
        "avf" => avf::avf_to_png(args.input, args.output),
        "batch" => batch(args.input, args.output),
        _ => panic!("Unknown operation!")
    }
    println!("Process completed in {:?}", timecheck.elapsed());
    thread::sleep(Duration::from_secs(1)); // cheap trick to ensure threads finish before the main one does
}

fn batch(input_dir: PathBuf, output: PathBuf){
    for entries in std::fs::read_dir(input_dir.clone()).unwrap() {
        let entry = entries.unwrap();
        if entry.path().extension().is_none() {
            println!{"File {:?} has no extension", entry};
            continue;
        }
        else {
            // borrow and clone data so that our thread can use it
            let outpath = output.clone();
            thread::spawn(move || {
                match entry.path().extension().unwrap().to_str().unwrap() {
                "avf" | "AVF" => {avf::avf_to_png(entry.path(),outpath.clone())}
                _ => {println!("Not an AVF file: {:?}", entry.path())}
                }
            });
            thread::sleep(Duration::from_micros(50)); // adjust as needed to prevent the "too many files open" std error
        }
    }
}