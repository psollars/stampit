use chrono::{Local, NaiveDateTime, TimeZone, Utc};
use clap::{Arg, Command};
use exif::{In, Reader, Tag, Value};
use std::fs::{metadata, File};
use std::io::BufReader;
use std::path::Path;

fn format_exif_date(file_path: &Path, date_format: &str) -> Option<String> {
    // Open the file and prepare it for EXIF metadata reading
    let file = File::open(file_path).ok()?;
    let mut bufreader = BufReader::new(file);

    // Parse EXIF metadata
    let reader = Reader::new();
    let exif_data = reader.read_from_container(&mut bufreader).ok()?;

    // Extract the DateTimeOriginal tag
    if let Some(field) = exif_data.get_field(Tag::DateTimeOriginal, In::PRIMARY) {
        if let Value::Ascii(ref vec) = field.value {
            if let Some(date_bytes) = vec.first() {
                let exif_date_str = String::from_utf8_lossy(date_bytes);

                // Parse the EXIF datetime string (format: %Y:%m:%d %H:%M:%S)
                if let Ok(parsed_date) =
                    NaiveDateTime::parse_from_str(&exif_date_str, "%Y:%m:%d %H:%M:%S")
                {
                    // Format the parsed date into the user-specified format
                    return Some(parsed_date.format(date_format).to_string());
                }
            }
        }
    }

    None
}

fn format_modified_date(file_path: &Path, date_format: &str) -> Option<String> {
    let metadata = metadata(file_path).ok()?;
    let modified_time = metadata.modified().ok()?;

    // Convert the system time to a UNIX timestamp
    let timestamp = modified_time
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;

    // Convert the UNIX timestamp to a DateTime<Utc>
    let date_time_utc = Utc.timestamp_opt(timestamp, 0).single()?;

    // Convert DateTime<Utc> to Local DateTime
    let local_date_time = date_time_utc.with_timezone(&Local);

    // Format the date into the user-specified format
    Some(local_date_time.format(date_format).to_string())
}

fn get_formatted_date(file_path: &Path, date_format: &str) -> Option<String> {
    if let Some(formatted_date) = format_exif_date(file_path, date_format) {
        Some(formatted_date)
    } else if let Some(modified_date) = format_modified_date(file_path, date_format) {
        Some(modified_date)
    } else {
        None
    }
}

fn collect_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            files.extend(collect_files(&path)?);
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

fn write_new_file_name(
    original_path: &Path,
    formatted_date: &str,
    verbose: bool,
) -> Result<(), std::io::Error> {
    if let Some(parent) = original_path.parent() {
        let mut new_file_name = format!(
            "{}.{}",
            formatted_date,
            original_path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
        );
        let mut new_path = parent.join(&new_file_name);

        // Add counter suffix if file already exists
        let mut counter = 1;
        while new_path.exists() && new_path != original_path {
            new_file_name = format!(
                "{}-{}.{}",
                formatted_date,
                counter,
                original_path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase()
            );
            new_path = parent.join(&new_file_name);
            counter += 1;
        }

        // Rename the file
        std::fs::rename(original_path, &new_path)?;

        if verbose {
            println!(
                "Renamed '{}' to '{}'",
                original_path.display(),
                new_path.display()
            );
        }
    }
    Ok(())
}

fn rename_files(
    path: &str,
    exif_only: bool,
    modified_only: bool,
    date_format: &str,
    verbose: bool,
    write: bool,
) {
    if verbose {
        println!("Path: {}", path);
        println!("EXIF Only: {}", exif_only);
        println!("Modified Only: {}", modified_only);
        println!("Date Format: {}", date_format);
        println!("Verbose: {}", verbose);
        println!("Write: {}", write);
    }

    let root_path = Path::new(path);
    if !root_path.exists() {
        eprintln!("Error: Path '{}' does not exist.", path);
        std::process::exit(1);
    }

    // Collect files into an array
    let files = if root_path.is_file() {
        vec![root_path.to_path_buf()]
    } else if root_path.is_dir() {
        match collect_files(root_path) {
            Ok(files) => files,
            Err(e) => {
                eprintln!("Error processing directory: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("Error: Path '{}' is neither a file nor a directory.", path);
        std::process::exit(1);
    };

    // Process each file
    for file_path in files {
        if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with('.') {
                continue; // Skip hidden files like .DS_Store
            }
        }

        let formatted_date = if exif_only {
            format_exif_date(&file_path, date_format)
        } else if modified_only {
            format_modified_date(&file_path, date_format)
        } else {
            get_formatted_date(&file_path, date_format)
        };

        if let Some(date) = formatted_date {
            if write {
                if let Err(e) = write_new_file_name(&file_path, &date, verbose) {
                    eprintln!("Error renaming file '{}': {}", file_path.display(), e);
                }
            } else if verbose {
                println!("Date for '{}': {}", file_path.display(), date);
            }
        } else {
            if verbose {
                println!(
                    "No date information available for '{}'.",
                    file_path.display()
                );
            }
        }
    }
}

fn main() {
    let matches = Command::new("stampit")
        .version("0.1.1")
        .author("Patrick Sollars <pjsollars@gmail.com>")
        .about("Rename files using EXIF or last modified date.")
        .arg(
            Arg::new("path")
                .help("Path to a file or directory.")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("exif_only")
                .short('e')
                .long("exif")
                .help("Only use EXIF date for renaming (cannot be used with --modified)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("modified_only")
                .short('m')
                .long("modified")
                .help("Only use modified date for renaming (cannot be used with --exif)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("date_format")
                .short('f')
                .long("format")
                .help("Specify a custom date format")
                .default_value("%Y-%m-%d_%H.%M.%S"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("write")
                .short('w')
                .long("write")
                .help("Rename files to the parsed date")
                .action(clap::ArgAction::SetTrue),
        )
        .group(
            clap::ArgGroup::new("date_source")
                .args(&["exif_only", "modified_only"])
                .multiple(false),
        )
        .get_matches();

    let path = matches.get_one::<String>("path").unwrap();
    let exif_only = matches.get_flag("exif_only");
    let modified_only = matches.get_flag("modified_only");
    let date_format = matches.get_one::<String>("date_format").unwrap();
    let verbose = matches.get_flag("verbose");
    let write = matches.get_flag("write");

    rename_files(path, exif_only, modified_only, date_format, verbose, write);
}
