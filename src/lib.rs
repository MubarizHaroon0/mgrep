use aho_corasick::AhoCorasick;
use colored::*;
use rayon::prelude::*;
use regex::Regex;
use std::error::Error;
use std::fs::{metadata, File};
use std::io::{BufRead, BufReader};
use std::path::Path;
use walkdir::WalkDir;

pub struct Config {
    pub query: String,
    pub filename: String,
    pub flag: String,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("not enough arguments");
        }
        let query = &args[1];
        let mut filename = "none".to_string();
        let mut flag = "none".to_string();
        if args.len() > 2 {
            if args[2] == "i" {
                flag = "i".to_string();
            } else {
                filename = args[2].clone();
                if args.len() > 3 {
                    flag = args[3].clone();
                }
            }
        }
        Ok(Config {
            query: query.to_string(),
            filename,
            flag,
        })
    }
}

pub fn search(query: &str, contents: &str, flag: &str) -> Vec<String> {
    let query = if flag == "i" {
        query.to_lowercase()
    } else {
        query.to_string()
    };
    let ac = AhoCorasick::new(&[&query]).expect("Failed to build AhoCorasick automaton");

    let re: Regex = if flag == "i" {
        Regex::new(&format!("(?i){}", regex::escape(&query))).expect("Invalid regex")
    } else {
        Regex::new(&regex::escape(&query)).expect("Failed to create regex")
    };
    contents
        .par_lines()
        .filter_map(|line| {
            let line_to_match = if flag == "i" {
                line.to_lowercase()
            } else {
                line.to_string()
            };
            if ac.is_match(&line_to_match) {
                Some(
                    re.replace_all(line, |caps: &regex::Captures| {
                        caps[0].blue().bold().to_string()
                    })
                    .to_string(),
                )
            } else {
                None
            }
        })
        .collect()
}

pub fn find_path(filename: &str, start_dir: &Path) -> Option<String> {
    WalkDir::new(start_dir)
        .into_iter()
        .par_bridge() // Convert to a parallel iterator
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_type().is_dir())
        .find_any(|e| e.file_name().to_string_lossy().contains(filename))
        .map(|e| e.path().to_string_lossy().into_owned())
}

pub fn find_all_paths(start_dir: &Path) -> Vec<String> {
    WalkDir::new(start_dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            !entry.file_type().is_dir() &&
metadata(entry.path()).is_ok() && // Check if metadata can be read
File::open(entry.path()).is_ok() // Check if the file can be opened
        })
        .map(|entry| entry.into_path().to_string_lossy().into_owned())
        .collect()
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let paths = if config.filename != "none" {
        if let Some(path) = find_path(&config.filename, Path::new("./")) {
            vec![path]
        } else {
            vec![]
        }
    } else {
        find_all_paths(Path::new("./"))
    };

    if paths.is_empty() {
        println!("No files found containing the query.");
        return Ok(());
    }

    for path in &paths {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let contents = reader
            .lines()
            .filter_map(Result::ok)
            .collect::<Vec<String>>()
            .join("\n");

        let result_lines = search(&config.query, &contents, &config.flag);
        if result_lines.len() > 0 {
            println!("In file: {}", path);
            for line in result_lines {
                println!("{}", line);
            }
        }
    }

    Ok(())
}
