use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

pub fn sanitize_bot_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

pub fn bot_file_path(dir: &Path, bot_name: &str) -> PathBuf {
    dir.join(format!("{}.jsonl", sanitize_bot_name(bot_name)))
}

pub fn append_line<T: Serialize>(dir: &Path, bot_name: &str, item: &T) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let path = bot_file_path(dir, bot_name);
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    let json = serde_json::to_string(item).map_err(io::Error::other)?;
    writeln!(file, "{json}")?;
    Ok(())
}

pub fn load_lines<T: DeserializeOwned>(dir: &Path, bot_name: &str) -> io::Result<Vec<T>> {
    let path = bot_file_path(dir, bot_name);
    let file = match File::open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let reader = BufReader::new(file);
    let mut items = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str(&line) {
            Ok(item) => items.push(item),
            Err(e) => eprintln!(
                "warning: skipping malformed line {i} in {}: {e}",
                path.display()
            ),
        }
    }
    Ok(items)
}

pub fn rewrite_lines<'a, T: Serialize + 'a>(
    dir: &Path,
    bot_name: &str,
    items: impl Iterator<Item = &'a T>,
) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let path = bot_file_path(dir, bot_name);
    let tmp_path = path.with_extension("jsonl.tmp");
    {
        let mut file = File::create(&tmp_path)?;
        for item in items {
            let json = serde_json::to_string(item).map_err(io::Error::other)?;
            writeln!(file, "{json}")?;
        }
    }
    fs::rename(&tmp_path, &path)?;
    Ok(())
}

pub fn remove_bot_file(dir: &Path, bot_name: &str) -> io::Result<()> {
    let path = bot_file_path(dir, bot_name);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn discover_bots(dir: &Path) -> io::Result<Vec<String>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("jsonl")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            names.push(stem.to_owned());
        }
    }
    Ok(names)
}
