use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::{Cursor, Result, SeekFrom, Take};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::result::Result::Ok;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::vec::Vec;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use fs2::FileExt;
use regex::Regex;

use data::{Entry, Hint};
use util::{xxhash32, XxHash32, get_file_handle};

const DATA_FILE_EXTENSION: &'static str = "cask.data";
const HINT_FILE_EXTENSION: &'static str = "cask.hint";
const LOCK_FILE_NAME: &'static str = "cask.lock";

const DEFAULT_SIZE_THRESHOLD: usize = 2000 * 1024 * 1024;

pub struct Log {
    pub path: PathBuf,
    sync: bool,
    size_threshold: usize,
    lock_file: File,
    files: Vec<u32>,
    current_file_id: AtomicUsize,
    pub active_file_id: u32,
    active_log_writer: LogWriter,
}

impl Log {
    pub fn open(path: &str, sync: bool) -> Result<Log> {
        let path = PathBuf::from(path);

        if path.exists() {
            assert!(path.is_dir());
        } else {
            fs::create_dir(&path)?;
        }

        let lock_file = File::create(path.join(LOCK_FILE_NAME))?;
        lock_file.try_lock_exclusive()?;

        let files = find_data_files(&path)?;

        let active_file_id = if files.is_empty() {
            0
        } else {
            files[files.len() - 1] + 1
        };

        let active_log_writer = LogWriter::new(&path, active_file_id, sync)?;

        info!("Created new active data file {:?}",
              active_log_writer.data_file_path);

        Ok(Log {
            path: path,
            sync: sync,
            size_threshold: DEFAULT_SIZE_THRESHOLD,
            lock_file: lock_file,
            files: files,
            current_file_id: AtomicUsize::new(active_file_id as usize),
            active_file_id: active_file_id,
            active_log_writer: active_log_writer,
        })
    }

    pub fn files(&self) -> Vec<u32> {
        self.files.clone()
    }

    pub fn entries<'a>(&self, file_id: u32) -> Result<Entries<'a>> {
        let data_file_path = get_data_file_path(&self.path, file_id);
        info!("Loading data file: {:?}", data_file_path);
        let data_file = get_file_handle(&data_file_path, false)?;
        let data_file_size = data_file.metadata()?.len();

        Ok(Entries {
            data_file: data_file.take(data_file_size),
            data_file_pos: 0,
            phantom: PhantomData,
        })
    }

    pub fn hints<'a>(&self, file_id: u32) -> Result<Option<Result<Hints<'a>>>> {
        let hint_file_path = get_hint_file_path(&self.path, file_id);
        if is_valid_hint_file(&hint_file_path)? {
            info!("Loading hint file: {:?}", hint_file_path);
            let hint_file = get_file_handle(&hint_file_path, false)?;
            let hint_file_size = hint_file.metadata()?.len();

            Ok(Some(Ok(Hints {
                hint_file: hint_file.take(hint_file_size - 4),
                phantom: PhantomData,
            })))
        } else {
            Ok(None)
        }
    }

    pub fn recreate_hints<'a>(&mut self, file_id: u32) -> Result<RecreateHints<'a>> {
        let hint_file_path = get_hint_file_path(&self.path, file_id);
        warn!("Re-creating hint file: {:?}", hint_file_path);

        let hint_writer = HintWriter::new(&self.path, file_id)?;
        let entries = self.entries(file_id)?;

        Ok(RecreateHints {
            hint_writer: hint_writer,
            entries: entries,
        })
    }

    pub fn read_entry<'a>(&self, file_id: u32, entry_pos: u64) -> Result<Entry<'a>> {
        let mut data_file = get_file_handle(&get_data_file_path(&self.path, file_id), false)?;
        data_file.seek(SeekFrom::Start(entry_pos))?;
        Entry::from_read(&mut data_file)
    }

    pub fn append_entry<'a>(&mut self, entry: &Entry<'a>) -> Result<(u32, u64)> {
        if self.active_log_writer.data_file_pos + entry.size() > self.size_threshold as u64 {
            info!("Active data file {:?} reached file limit",
                  self.active_log_writer.data_file_path);

            self.new_active_writer()?;
        }

        let entry_pos = self.active_log_writer.write(entry)?;

        Ok((self.active_file_id, entry_pos))
    }

    pub fn new_file_id(&self) -> u32 {
        self.current_file_id.fetch_add(1, Ordering::SeqCst) as u32 + 1
    }

    pub fn swap_file(&mut self, file_id: u32, new_file_id: u32) -> Result<()> {
        // FIXME: convert to error
        let idx = self.files.binary_search(&file_id).unwrap();
        self.files.remove(idx);

        self.add_file(new_file_id);

        let data_file_path = get_data_file_path(&self.path, file_id);
        let hint_file_path = get_hint_file_path(&self.path, file_id);

        fs::remove_file(data_file_path)?;
        fs::remove_file(hint_file_path)?;

        Ok(())
    }

    pub fn add_file(&mut self, file_id: u32) {
        self.files.push(file_id);
        self.files.sort();
    }

    fn new_active_writer(&mut self) -> Result<()> {
        let active_file_id = self.active_file_id;
        self.add_file(active_file_id);

        info!("Closed active data file {:?}",
              self.active_log_writer.data_file_path);

        self.active_file_id = self.new_file_id();
        self.active_log_writer = LogWriter::new(&self.path, self.active_file_id, self.sync)?;

        info!("Created new active data file {:?}",
              self.active_log_writer.data_file_path);

        Ok(())
    }
}

impl Drop for Log {
    fn drop(&mut self) {
        let _ = self.lock_file.unlock();
    }
}

pub struct LogWriter {
    sync: bool,
    data_file_path: PathBuf,
    data_file: File,
    data_file_pos: u64,
    hint_writer: HintWriter,
}

impl LogWriter {
    pub fn new(path: &Path, file_id: u32, sync: bool) -> Result<LogWriter> {
        let data_file_path = get_data_file_path(path, file_id);
        let data_file = get_file_handle(&data_file_path, true)?;

        let hint_writer = HintWriter::new(path, file_id)?;

        Ok(LogWriter {
            sync: sync,
            data_file_path: data_file_path,
            data_file: data_file,
            data_file_pos: 0,
            hint_writer: hint_writer,
        })
    }

    pub fn write<'a>(&mut self, entry: &Entry<'a>) -> Result<u64> {
        let entry_pos = self.data_file_pos;

        let hint = Hint::new(entry, entry_pos);
        entry.write_bytes(&mut self.data_file)?;

        self.hint_writer.write(&hint)?;

        if self.sync {
            self.data_file.sync_data()?;
        }

        self.data_file_pos += entry.size();

        Ok(entry_pos)
    }
}

impl Drop for LogWriter {
    fn drop(&mut self) {
        if self.sync {
            let _ = self.data_file.sync_data();
        }
    }
}

struct HintWriter {
    hint_file: File,
    hint_file_hasher: XxHash32,
}

impl HintWriter {
    pub fn new(path: &Path, file_id: u32) -> Result<HintWriter> {
        let hint_file = get_file_handle(&get_hint_file_path(path, file_id), true)?;

        Ok(HintWriter {
            hint_file: hint_file,
            hint_file_hasher: XxHash32::new(),
        })
    }

    pub fn write<'a>(&mut self, hint: &Hint<'a>) -> Result<()> {
        hint.write_bytes(&mut self.hint_file)?;
        hint.write_bytes(&mut self.hint_file_hasher)?;
        Ok(())
    }
}

impl Drop for HintWriter {
    fn drop(&mut self) {
        let _ = self.hint_file.write_u32::<LittleEndian>(self.hint_file_hasher.get());
    }
}

pub struct Entries<'a> {
    data_file: Take<File>,
    data_file_pos: u64,
    phantom: PhantomData<&'a ()>,
}

impl<'a> Iterator for Entries<'a> {
    type Item = (u64, Result<Entry<'a>>);

    fn next(&mut self) -> Option<(u64, Result<Entry<'a>>)> {
        let limit = self.data_file.limit();
        if limit == 0 {
            None
        } else {
            let entry = Entry::from_read(&mut self.data_file);
            let entry_pos = self.data_file_pos;

            let read = limit - self.data_file.limit();

            self.data_file_pos += read;

            let entry = match entry {
                Ok(entry) => {
                    assert_eq!(entry.size(), read);
                    Ok(entry)
                }
                e => e,
            };

            Some((entry_pos, entry))
        }
    }
}

pub struct Hints<'a> {
    hint_file: Take<File>,
    phantom: PhantomData<&'a ()>,
}

impl<'a> Iterator for Hints<'a> {
    type Item = Result<Hint<'a>>;

    fn next(&mut self) -> Option<Result<Hint<'a>>> {
        if self.hint_file.limit() == 0 {
            None
        } else {
            Some(Hint::from_read(&mut self.hint_file))
        }
    }
}

pub struct RecreateHints<'a> {
    hint_writer: HintWriter,
    entries: Entries<'a>,
}

impl<'a> Iterator for RecreateHints<'a> {
    type Item = Result<Hint<'a>>;

    fn next(&mut self) -> Option<Result<Hint<'a>>> {
        self.entries.next().map(|e| {
            let (entry_pos, entry) = e;
            let hint = Hint::from(entry?, entry_pos);
            self.hint_writer.write(&hint)?;
            Ok(hint)
        })
    }
}

impl<'a> Drop for RecreateHints<'a> {
    fn drop(&mut self) {
        while self.next().is_some() {}
    }
}

fn get_data_file_path(path: &Path, file_id: u32) -> PathBuf {
    let file_id = format!("{:010}", file_id);
    path.join(file_id).with_extension(DATA_FILE_EXTENSION)
}

fn get_hint_file_path(path: &Path, file_id: u32) -> PathBuf {
    let file_id = format!("{:010}", file_id);
    path.join(file_id).with_extension(HINT_FILE_EXTENSION)
}

fn find_data_files(path: &Path) -> Result<Vec<u32>> {
    let files = fs::read_dir(path)?;

    lazy_static! {
        static ref RE: Regex =
            Regex::new(&format!("(\\d+).{}$", DATA_FILE_EXTENSION)).unwrap();
    }

    let mut data_files = Vec::new();

    for file in files {
        let file = file?;
        if file.metadata()?.is_file() {
            let file_name = file.file_name();
            let captures = RE.captures(file_name.to_str().unwrap());
            if let Some(n) = captures.and_then(|c| c.at(1).and_then(|n| n.parse::<u32>().ok())) {
                data_files.push(n)
            }
        }
    }

    data_files.sort();

    Ok(data_files)
}

fn is_valid_hint_file(path: &Path) -> Result<bool> {
    Ok(path.is_file() &&
       {
        let mut hint_file = get_file_handle(path, false)?;

        // FIXME: avoid reading the whole hint file into memory;
        let mut buf = Vec::new();
        hint_file.read_to_end(&mut buf)?;

        buf.len() >= 4 &&
        {
            let hash = xxhash32(&buf[..buf.len() - 4]);

            let mut cursor = Cursor::new(&buf[buf.len() - 4..]);
            let checksum = cursor.read_u32::<LittleEndian>()?;

            let valid = hash == checksum;

            if !valid {
                warn!("Found corrupt hint file: {:?}", &path);
            }

            valid
        }
    })
}
