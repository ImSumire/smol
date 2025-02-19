use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write}
};

use chrono::Utc;
use jwalk::WalkDir;

const REMOVED_SYMBOL: char = ' ';
const JOURNAL_EXT: &str = "md";

fn main() {
    let journal_dir_str = "~/.local/share/smoljournals";  // argument
    let root = "/home/";  // argument (default: home)
    let description = "Bite";  // argument (default: date)

    let journal_dir_string = shellexpand::full(journal_dir_str)
        .unwrap()
        .to_string();
    let journal_dir = journal_dir_string.as_str();

    let mut existing_files = HashSet::new();

    if fs::metadata(journal_dir).is_ok() {
        // Load older journals (safe if empty)
        for entry in WalkDir::new(journal_dir).sort(true) {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() &&
                    entry.path().extension().unwrap().eq(JOURNAL_EXT) {
                    let mut file = File::open(entry.path()).unwrap();
                    let mut content = String::new();
                    file.read_to_string(&mut content).unwrap();
    
                    for line in content.lines() {
                        if !line.is_empty() {
                            match line.as_bytes()[0] as char {
                                '#' => {}
    
                                REMOVED_SYMBOL => {
                                    let path = &line[1..];
                                    existing_files.remove(path);
                                }
    
                                _ => {
                                    existing_files.insert(line.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    else {
        fs::create_dir_all(journal_dir).unwrap();
        // No older journals, don't load them
    }

    let now = Utc::now();
    let datetime = now.format("%Y-%m-%d_%H:%M:%S");
    let journal_path = format!("{journal_dir}/{datetime}.{JOURNAL_EXT}");
    let mut journal = File::create(journal_path).unwrap();

    journal
        .write_all(format!("# ({datetime}) {description}\n\n").as_bytes())
        .unwrap();

    for entry in WalkDir::new(root).sort(true) {
        if let Ok(entry) = entry {
            if entry.file_type().is_file() {
                let path = entry.path().display().to_string();

                if !existing_files.contains(&path) {
                    journal.write_all(path.as_bytes()).unwrap();
                    journal.write_all(b"\n").unwrap();
                } else {
                    existing_files.remove(&path);
                }
            }
        }
    }

    if !existing_files.is_empty() {
        journal.write_all("\n# Deleted\n".as_bytes()).unwrap();
    }
    for path in existing_files.iter() {
        let removed = format!("{REMOVED_SYMBOL}{path}");
        journal.write_all(removed.as_bytes()).unwrap();
        journal.write_all(b"\n").unwrap();
    }

    println!("\x1b[32;1m✓\x1b[0m Journal saved at {journal_dir_str}/{datetime}.{JOURNAL_EXT} (\x1b]8;;file://{journal_dir}/{datetime}.{JOURNAL_EXT}\x1b\\click to open\x1b]8;;\x1b\\)");
}
