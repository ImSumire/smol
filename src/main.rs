use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Error, Write}, path::Path, time::Instant
};

use chrono::Utc;
use clap::{builder::{styling, Styles}, Parser};
use jwalk::WalkDir;

const REMOVED_SYMBOL: char = ' ';
const JOURNAL_EXT: &str = "md";


/*
  /)/)
( . .)
( づ♥
*/


#[derive(Parser, Debug)]
#[command(
    version,
    about = "\x1b[30m
    /)/)
  ( . .)
  ( づ♥ \x1b[0m
Here's \x1b[31msmol\x1b[0m, your disk buddy!",
    long_about = None,
    styles = Styles::styled()
        .header(styling::AnsiColor::Magenta.on_default().bold())
        .usage(styling::AnsiColor::Magenta.on_default().bold())
        .literal(styling::AnsiColor::White.on_default().bold())
        .placeholder(styling::AnsiColor::Black.on_default().bold())
        .error(styling::AnsiColor::Red.on_default().bold())
        .valid(styling::AnsiColor::Green.on_default().bold())
        .invalid(styling::AnsiColor::Yellow.on_default().bold())
)]
struct Args {
    /// Description of the new journal entry
    #[arg(default_value = "")]
    description: String,

    /// Output directory for the journals
    #[arg(short = 'o', long, default_value = "~/.local/share/smoljournals")]
    output: String,

    /// Root directory to scan for files
    #[arg(short = 'r', long, default_value = "~")]
    root: String,
}


fn get_existing_files(journal_dir: &Path) -> Result<BTreeSet<String>, Error> {
    let mut files = BTreeSet::new();

    if fs::metadata(journal_dir).is_ok() {
        // Load older journals (safe if empty), need to be sorted
        for entry in WalkDir::new(journal_dir).sort(true) {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() &&
                    entry.path().extension().unwrap().eq(JOURNAL_EXT) {

                    let file = File::open(entry.path())?;
                    let reader = BufReader::new(file);
    
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            if !line.is_empty() {
                                match line.as_bytes()[0] as char {
                                    '#' => {}
        
                                    REMOVED_SYMBOL => {
                                        let path = &line[1..];
                                        files.remove(path);
                                    }
        
                                    _ => {
                                        files.insert(line.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    else {
        fs::create_dir_all(journal_dir)?;
        // No older journals, don't load them
    }

    Ok(files)
}

fn main() -> Result<(), Error> {
    let start = Instant::now();

    // Get arguments
    let args = Args::parse();

    let output_str = shellexpand::full(&args.output).unwrap().into_owned();
    let output = Path::new(output_str.as_str());

    let root_str = shellexpand::full(&args.root).unwrap().into_owned();
    let root = Path::new(root_str.as_str());

    let description = args.description;

    // Get the existing files from the past journals
    let mut existing_files = get_existing_files(&output)?;

    // Get time, yeah two sys calls :/
    let now = Utc::now();
    let datetime = now.format("%Y-%m-%d_%H:%M:%S");

    // Create the journal
    let journal_path = output.join(format!("{datetime}.{JOURNAL_EXT}"));
    
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&journal_path)?;

    let mut writer = BufWriter::new(file);

    // Add the description
    writeln!(writer, "# ({datetime}) {description}")?;

    // List the added files
    for entry in WalkDir::new(root) {
        if let Ok(entry) = entry {
            // Ignore directories and symlinks
            if entry.file_type().is_file() {
                let path = entry.path().display().to_string();

                // O(1)
                if !existing_files.contains(&path) {
                    writeln!(writer, "{}", path)?;
                } else {
                    existing_files.remove(&path);
                }
            }
        }
    }

    // List the deleted files
    if !existing_files.is_empty() {
        writeln!(writer, "\n# Deleted")?;

        for path in existing_files.iter() {
            writeln!(writer, "{REMOVED_SYMBOL}{path}")?;
        }
    }

    let duration = start.elapsed();
    let path = journal_path.display();
    println!("\x1b[35mFinished\x1b[0m Journal done in {duration:?} \
\x1b[2m(\x1b]8;;file://{path}\x1b\\{path}\x1b]8;;\x1b\\)\x1b[0m");

    Ok(())
}
