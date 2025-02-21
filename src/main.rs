use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Error, Write}, path::Path, time::Instant
};

use chrono::Utc;
use clap::{builder::{styling, Styles}, Parser};
use humansize::{FormatSize, DECIMAL};
use jwalk::WalkDir;

const REMOVED_SYMBOL: char = ' ';
const JOURNAL_EXT: &str = "md";

static mut COMPRESSIBLE_FILES: u32 = 0;
static mut POSSIBLY_SAVED_BYTES_LOSSLESS: usize = 0;
static mut POSSIBLY_SAVED_BYTES_LOSSY: usize = 0;

static mut USELESS_FILES: u32 = 0;
static mut POSSIBLY_SAVED_BYTES_USELESS: usize = 0;


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
    #[arg(short = 'o', default_value = "~/.local/share/smoljournals")]
    output: String,

    /// Root directory to scan for files
    #[arg(short = 'r', default_value = "~")]
    root: String,

    /// Full check mode
    #[arg(short = 'f')]
    full: bool,

    // Give add/remove stats
    #[arg(short = 's')]
    stats: bool,
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

fn add_save_approx(path: &str, lossless: f32, lossy: f32) {
    if let Ok(metadata) = fs::metadata(path) {
        let size = metadata.len();

        let approx = (size as f32 * lossless) as usize;
        unsafe { POSSIBLY_SAVED_BYTES_LOSSLESS += approx; };

        let approx = (size as f32 * lossy) as usize;
        unsafe { POSSIBLY_SAVED_BYTES_LOSSY += approx; };
    }
}

fn handle_path(path: &str) {
    if let Some(ext) = Path::new(path).extension() {
        if let Some(ext) = ext.to_str() {
            match ext {
                "mp3" => {
                    unsafe { COMPRESSIBLE_FILES += 1; };
                    add_save_approx(path, 0.1, 0.5);
                }

                "jpeg" | "jpg" | "webp" | "png" | "gif" | "svg" => {
                    unsafe { COMPRESSIBLE_FILES += 1; };
                    add_save_approx(path, 0.4, 0.7);
                }

                "mp4" | "av1" | "webm" => {
                    unsafe { COMPRESSIBLE_FILES += 1; };
                    add_save_approx(path, 0.4, 0.7);
                }

                "pdf" | "docx" | "xlsx" | "pptx" => {
                    unsafe { COMPRESSIBLE_FILES += 1; };
                    add_save_approx(path, 0.4, 0.7);
                }

                "tmp" |
                "temp" |
                "deb" |
                "old" |
                "~" |
                "log" |
                "dmp" |
                "crdownload" |
                "part" |
                "download" |
                "opdownload" |
                "pyc" |
                "pyo" |
                "o" |
                "so" => {
                    unsafe { USELESS_FILES += 1; };

                    if let Ok(metadata) = fs::metadata(path) {
                        let size = metadata.len();
                        unsafe { POSSIBLY_SAVED_BYTES_USELESS += size as usize; };
                    }
                }

                _ => {}
            }
        }
    }
}

// Not optimized
fn get_dir_size(path: &Path) -> u64 {
    WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| fs::metadata(entry.path()).ok().map(|m| m.len()))
        .sum()
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

    let full = args.full;

    let stats = args.stats;

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
    let mut added: usize= 0;
    for entry in WalkDir::new(root) {
        if let Ok(entry) = entry {
            // Ignore directories and symlinks
            if entry.file_type().is_file() {
                let path = entry.path().display().to_string();
                if full {
                    handle_path(&path);
                }

                // O(1)
                if !existing_files.contains(&path) {
                    writeln!(writer, "{}", path)?;
                    added += 1;
                } else {
                    existing_files.remove(&path);
                }
            }
        }
    }

    // List the deleted files
    let removed = existing_files.len();
    if !existing_files.is_empty() {
        writeln!(writer, "\n# Deleted")?;

        for path in existing_files.iter() {
            writeln!(writer, "{REMOVED_SYMBOL}{path}")?;
        }
    }

    let path = journal_path.display();
    let duration = start.elapsed();
    println!("    \x1b[35mFinished\x1b[0m Journal done in {duration:?} \
\x1b[2m(\x1b]8;;file://{path}\x1b\\{path}\x1b]8;;\x1b\\)\x1b[0m");

    if stats {
        println!("\x1b[35m       Stats\x1b[0m {added} files added");
        println!("\x1b[35m       Stats\x1b[0m {removed} files removed");
    }

    if full {
        println!(
            "\x1b[35m       Stats\x1b[0m Compressible files: {} (lossless: {}, lossy: {})",
            unsafe { COMPRESSIBLE_FILES },
            unsafe { POSSIBLY_SAVED_BYTES_LOSSLESS }.format_size(DECIMAL),
            unsafe { POSSIBLY_SAVED_BYTES_LOSSY }.format_size(DECIMAL)
        );
        
        println!(
            "       \x1b[35mStats\x1b[0m Useless files:\x1b[0m {} ({})",
            unsafe { USELESS_FILES },
            unsafe { POSSIBLY_SAVED_BYTES_USELESS }.format_size(DECIMAL)
        );

        if let Ok(trash_path) = shellexpand::full("~/.local/share/Trash/files") {
            let trash_size = get_dir_size(
                Path::new(trash_path.into_owned().as_str())
            );

            if trash_size > 0 {
                println!("       \x1b[35mStats\x1b[0m Trash:\x1b[0m {}", trash_size.format_size(DECIMAL));
                println!("     \x1b[35mSuggest\x1b[0m Empty your Trash");
            }
        }

        if let Ok(cache_path) = shellexpand::full("~/.cache") {
            let cache_size = get_dir_size(
                Path::new(cache_path.into_owned().as_str())
            );

            if cache_size > 0 {
                println!("       \x1b[35mStats\x1b[0m .cache:\x1b[0m {}", cache_size.format_size(DECIMAL));
                println!("     \x1b[35mSuggest\x1b[0m Empty your .cache");
            }
        }
        
        println!("     \x1b[35mSuggest\x1b[0m Run `fdupes -r ~` (`-mr` to get the total size)");
        println!("     \x1b[35mSuggest\x1b[0m Run `sudo apt autoremove --purge`");
    }

    Ok(())
}
