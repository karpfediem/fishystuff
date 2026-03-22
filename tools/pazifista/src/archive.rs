use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use flate2::read::ZlibDecoder;

use crate::compression::decompress_bdo;
use crate::ice::IceKey;
use crate::wildcard::wild_match_any;

const BDO_DECRYPTION_KEY: [u8; 8] = [0x51, 0xF3, 0x0F, 0x11, 0x04, 0x24, 0x6A, 0x00];

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub paz_num: u32,
    pub offset: u32,
    pub compressed_size: u32,
    pub original_size: u32,
    pub file_name: String,
    pub file_path: String,
}

#[derive(Clone, Copy, Debug)]
pub struct ExtractOptions {
    pub quiet: bool,
    pub no_folders: bool,
    pub yes_to_all: bool,
}

#[derive(Default)]
struct ExtractState {
    overwrite_files: bool,
    rename_files: bool,
    create_paths: bool,
}

pub struct ArchiveIndex {
    archive_path: PathBuf,
    entries: Vec<FileEntry>,
    mobile: bool,
    paz_names: HashMap<u32, PathBuf>,
    directory_index: Option<HashMap<String, PathBuf>>,
}

impl ArchiveIndex {
    pub fn from_meta(path: &Path, quiet: bool) -> Result<Self> {
        let mut file = File::open(path)
            .with_context(|| format!("failed to open meta archive {}", path.display()))?;

        let client_version = read_u32_from(&mut file)?;
        let paz_count = read_u32_from(&mut file)?;
        file.seek(SeekFrom::Current(i64::from(paz_count) * 12))
            .with_context(|| format!("failed to skip PAZ table in {}", path.display()))?;

        let files_count = read_u32_from(&mut file)?;
        if !quiet {
            println!();
            println!("Client version: {client_version}");
            println!("Number of stored files: {files_count}");
        }

        let files_info = read_exact_vec(&mut file, files_count as usize * 28)
            .context("failed to read meta file table")?;

        let folder_names_length = read_u32_from(&mut file)? as usize;
        let folder_names_encrypted = read_exact_vec(&mut file, folder_names_length)
            .context("failed to read meta folder names")?;
        let mobile = folder_names_encrypted
            .get(8..11)
            .map(|bytes| bytes == b"res")
            .unwrap_or(false);
        let folder_names_decrypted = if mobile {
            folder_names_encrypted
        } else {
            IceKey::thin(BDO_DECRYPTION_KEY).decrypt_buffer(&folder_names_encrypted)?
        };
        let folder_names =
            parse_meta_folder_names(&folder_names_decrypted).context("failed to parse folders")?;

        if !quiet {
            println!("Number of stored folder names: {}", folder_names.len());
        }

        let file_names_length = read_u32_from(&mut file)? as usize;
        let file_names_raw = read_exact_vec(&mut file, file_names_length)
            .context("failed to read meta file names")?;
        let file_names_decrypted = if mobile {
            file_names_raw
        } else {
            IceKey::thin(BDO_DECRYPTION_KEY).decrypt_buffer(&file_names_raw)?
        };
        let file_names =
            parse_cstring_table(&file_names_decrypted).context("failed to parse file names")?;

        if !quiet {
            println!("Number of stored file names: {}", file_names.len());
            println!();
        }

        let mut entries = Vec::with_capacity(files_count as usize);
        for chunk in files_info.chunks_exact(28) {
            let folder_num = le_u32(chunk, 4)?;
            let file_num = le_u32(chunk, 8)?;
            let paz_num = le_u32(chunk, 12)?;
            let file_name = file_names
                .get(file_num as usize)
                .with_context(|| format!("file name index {file_num} is out of bounds"))?
                .clone();
            let folder_name = folder_names
                .get(folder_num as usize)
                .with_context(|| format!("folder name index {folder_num} is out of bounds"))?
                .clone();

            entries.push(FileEntry {
                paz_num,
                offset: le_u32(chunk, 16)?,
                compressed_size: le_u32(chunk, 20)?,
                original_size: le_u32(chunk, 24)?,
                file_name: file_name.clone(),
                file_path: format!("{folder_name}{file_name}"),
            });
        }

        Ok(Self {
            archive_path: path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
            entries,
            mobile,
            paz_names: HashMap::new(),
            directory_index: None,
        })
    }

    pub fn from_paz(path: &Path, quiet: bool) -> Result<Self> {
        let mut file = File::open(path)
            .with_context(|| format!("failed to open PAZ archive {}", path.display()))?;
        let ui_paz_num = parse_paz_number(path)?;
        let paz_hash = read_u32_from(&mut file)?;
        let files_count = read_u32_from(&mut file)?;
        let names_length = read_u32_from(&mut file)? as usize;

        if !quiet {
            println!();
            println!("PAZ hash: {paz_hash}");
            println!("Number of stored files: {files_count}");
            println!();
        }

        let files_info = read_exact_vec(&mut file, files_count as usize * 24)
            .context("failed to read PAZ file table")?;
        let names_raw =
            read_exact_vec(&mut file, names_length).context("failed to read PAZ file names")?;

        let mobile = names_raw
            .get(0..3)
            .map(|bytes| bytes == b"res")
            .unwrap_or(false);
        let names_decrypted = if mobile {
            names_raw
        } else {
            IceKey::thin(BDO_DECRYPTION_KEY).decrypt_buffer(&names_raw)?
        };
        let names =
            parse_cstring_table(&names_decrypted).context("failed to parse PAZ name table")?;

        let mut entries = Vec::with_capacity(files_count as usize);
        for chunk in files_info.chunks_exact(24) {
            let folder_num = le_u32(chunk, 4)?;
            let file_num = le_u32(chunk, 8)?;
            let file_name = names
                .get(file_num as usize)
                .with_context(|| format!("file name index {file_num} is out of bounds"))?
                .clone();
            let folder_name = names
                .get(folder_num as usize)
                .with_context(|| format!("folder name index {folder_num} is out of bounds"))?
                .clone();

            entries.push(FileEntry {
                paz_num: ui_paz_num,
                offset: le_u32(chunk, 12)?,
                compressed_size: le_u32(chunk, 16)?,
                original_size: le_u32(chunk, 20)?,
                file_name: file_name.clone(),
                file_path: format!("{folder_name}{file_name}"),
            });
        }

        Ok(Self {
            archive_path: path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
            entries,
            mobile,
            paz_names: HashMap::new(),
            directory_index: None,
        })
    }

    pub fn list(&self, masks: &[String], quiet: bool) -> u32 {
        let mut counter = 0u32;
        for entry in &self.entries {
            if wild_match_any(masks, &entry.file_path) {
                println!(
                    "[{}] {} (size: {})",
                    paz_name(entry.paz_num),
                    entry.file_path,
                    entry.original_size
                );
                counter += 1;
            }
        }

        if !quiet {
            println!();
            println!(
                "Listed files: {counter}, total files: {}",
                self.entries.len()
            );
        }

        counter
    }

    pub fn extract(
        &mut self,
        masks: &[String],
        output_path: &Path,
        options: ExtractOptions,
    ) -> Result<u32> {
        let mut state = ExtractState {
            overwrite_files: options.yes_to_all,
            rename_files: false,
            create_paths: options.yes_to_all,
        };

        if !output_path.exists() {
            if state.create_paths {
                fs::create_dir_all(output_path).with_context(|| {
                    format!(
                        "failed to create output directory {}",
                        output_path.display()
                    )
                })?;
            } else {
                match prompt_create_output_path(output_path)? {
                    OutputPathDecision::Create => {
                        fs::create_dir_all(output_path).with_context(|| {
                            format!(
                                "failed to create output directory {}",
                                output_path.display()
                            )
                        })?
                    }
                    OutputPathDecision::Abort => return Ok(0),
                }
            }
        }

        let mut counter = 0u32;
        for entry in self.entries.clone() {
            if !wild_match_any(masks, &entry.file_path) {
                continue;
            }

            let requested_path = if options.no_folders {
                output_path.join(&entry.file_name)
            } else {
                output_path.join(&entry.file_path)
            };

            match self.extract_entry(&entry, requested_path, &mut state)? {
                ExtractOutcome::Extracted => {
                    if !options.quiet {
                        println!("> {} (size: {})", entry.file_path, entry.original_size);
                    }
                    counter += 1;
                }
                ExtractOutcome::Skipped => {}
                ExtractOutcome::Aborted => break,
            }
        }

        if !options.quiet {
            println!();
            println!(
                "Extracted files: {counter}, total files: {}",
                self.entries.len()
            );
        }

        Ok(counter)
    }

    fn extract_entry(
        &mut self,
        entry: &FileEntry,
        mut target_path: PathBuf,
        state: &mut ExtractState,
    ) -> Result<ExtractOutcome> {
        if let Some(parent) = target_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                if state.create_paths {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("failed to create {}", parent.display()))?;
                } else {
                    match prompt_create_path(parent)? {
                        CreatePathDecision::Yes => fs::create_dir_all(parent)
                            .with_context(|| format!("failed to create {}", parent.display()))?,
                        CreatePathDecision::Always => {
                            fs::create_dir_all(parent).with_context(|| {
                                format!("failed to create {}", parent.display())
                            })?;
                            state.create_paths = true;
                        }
                        CreatePathDecision::Skip => return Ok(ExtractOutcome::Skipped),
                        CreatePathDecision::Abort => return Ok(ExtractOutcome::Aborted),
                    }
                }
            }
        }

        if target_path.exists() && !state.overwrite_files {
            if state.rename_files {
                target_path = auto_rename_path(&target_path);
            } else {
                match prompt_file_conflict(&target_path)? {
                    FileConflictDecision::Overwrite => {}
                    FileConflictDecision::Rename => {
                        target_path = auto_rename_path(&target_path);
                    }
                    FileConflictDecision::OverwriteAll => {
                        state.overwrite_files = true;
                    }
                    FileConflictDecision::RenameAll => {
                        target_path = auto_rename_path(&target_path);
                        state.rename_files = true;
                    }
                    FileConflictDecision::Abort => return Ok(ExtractOutcome::Aborted),
                }
            }
        }

        let payload = self.read_payload(entry)?;
        fs::write(&target_path, payload)
            .with_context(|| format!("failed to write {}", target_path.display()))?;
        Ok(ExtractOutcome::Extracted)
    }

    fn read_payload(&mut self, entry: &FileEntry) -> Result<Vec<u8>> {
        if entry.compressed_size == 0 {
            return Ok(Vec::new());
        }

        if !self.mobile && entry.compressed_size % 8 != 0 {
            bail!(
                "invalid compressed size {} for {}",
                entry.compressed_size,
                entry.file_path
            );
        }

        let paz_path = self.resolve_paz_name(entry.paz_num)?;
        let mut file = File::open(&paz_path)
            .with_context(|| format!("failed to open {}", paz_path.display()))?;
        file.seek(SeekFrom::Start(entry.offset as u64))
            .with_context(|| format!("failed to seek {}", paz_path.display()))?;

        let encrypted = read_exact_vec(&mut file, entry.compressed_size as usize)
            .with_context(|| format!("failed to read payload from {}", paz_path.display()))?;

        if self.mobile {
            return self.read_mobile_payload(entry, encrypted);
        }

        let decrypted = IceKey::thin(BDO_DECRYPTION_KEY).decrypt_buffer(&encrypted)?;
        let mut payload = if has_valid_bdo_header(&decrypted, entry.original_size) {
            decompress_bdo(&decrypted)?
        } else {
            decrypted
        };

        if payload.len() < entry.original_size as usize {
            bail!(
                "payload for {} is shorter than expected ({} < {})",
                entry.file_path,
                payload.len(),
                entry.original_size
            );
        }
        payload.truncate(entry.original_size as usize);
        Ok(payload)
    }

    fn read_mobile_payload(&self, entry: &FileEntry, raw: Vec<u8>) -> Result<Vec<u8>> {
        if entry.original_size == entry.compressed_size {
            return Ok(raw);
        }

        let mut decoder = ZlibDecoder::new(&raw[..]);
        let mut payload = Vec::with_capacity(entry.original_size as usize);
        decoder
            .read_to_end(&mut payload)
            .with_context(|| format!("failed to zlib-decompress {}", entry.file_path))?;

        if payload.len() != entry.original_size as usize {
            bail!(
                "zlib output size mismatch for {} ({} != {})",
                entry.file_path,
                payload.len(),
                entry.original_size
            );
        }

        Ok(payload)
    }

    fn resolve_paz_name(&mut self, paz_num: u32) -> Result<PathBuf> {
        if let Some(path) = self.paz_names.get(&paz_num) {
            return Ok(path.clone());
        }

        let filename = paz_name(paz_num);
        let lowercase = filename.to_ascii_lowercase();
        let candidate = self.archive_path.join(&filename);
        let resolved = if candidate.exists() {
            candidate
        } else {
            let directory_index = self.directory_index.get_or_insert_with(|| {
                let mut index = HashMap::new();
                if let Ok(entries) = fs::read_dir(&self.archive_path) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            index.insert(name.to_ascii_lowercase(), entry.path());
                        }
                    }
                }
                index
            });

            directory_index
                .get(&lowercase)
                .cloned()
                .unwrap_or_else(|| self.archive_path.join(&filename))
        };

        self.paz_names.insert(paz_num, resolved.clone());
        Ok(resolved)
    }
}

#[derive(Clone, Copy)]
enum ExtractOutcome {
    Extracted,
    Skipped,
    Aborted,
}

enum OutputPathDecision {
    Create,
    Abort,
}

enum CreatePathDecision {
    Yes,
    Always,
    Skip,
    Abort,
}

enum FileConflictDecision {
    Overwrite,
    Rename,
    OverwriteAll,
    RenameAll,
    Abort,
}

fn read_exact_vec(reader: &mut File, len: usize) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; len];
    reader.read_exact(&mut buffer)?;
    Ok(buffer)
}

fn read_u32_from(reader: &mut File) -> Result<u32> {
    let mut bytes = [0u8; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn le_u32(buffer: &[u8], offset: usize) -> Result<u32> {
    let bytes = buffer
        .get(offset..offset + 4)
        .ok_or_else(|| anyhow::anyhow!("truncated u32 at offset {offset}"))?;
    Ok(u32::from_le_bytes(bytes.try_into().unwrap()))
}

fn parse_meta_folder_names(buffer: &[u8]) -> Result<Vec<String>> {
    let mut offset = 0usize;
    let mut names = Vec::new();
    let end = buffer.len().saturating_sub(8);

    while offset < end {
        offset += 8;
        let (name, next_offset) = read_cstring(buffer, offset)?;
        names.push(name);
        offset = next_offset;
    }

    Ok(names)
}

fn parse_cstring_table(buffer: &[u8]) -> Result<Vec<String>> {
    let mut offset = 0usize;
    let mut names = Vec::new();
    while offset < buffer.len() {
        let (name, next_offset) = read_cstring(buffer, offset)?;
        names.push(name);
        offset = next_offset;
    }
    Ok(names)
}

fn read_cstring(buffer: &[u8], offset: usize) -> Result<(String, usize)> {
    let Some(relative_end) = buffer[offset..].iter().position(|&byte| byte == 0) else {
        bail!("missing NUL terminator in string table");
    };
    let end = offset + relative_end;
    let name = String::from_utf8_lossy(&buffer[offset..end]).into_owned();
    Ok((name, end + 1))
}

fn parse_paz_number(path: &Path) -> Result<u32> {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| anyhow::anyhow!("PAZ file name is not valid UTF-8"))?;
    if stem.len() < 4 {
        bail!("PAZ file name {} is too short", path.display());
    }
    stem[3..]
        .parse::<u32>()
        .with_context(|| format!("failed to parse PAZ number from {}", path.display()))
}

fn has_valid_bdo_header(buffer: &[u8], original_size: u32) -> bool {
    if buffer.len() <= 9 {
        return false;
    }
    if buffer[0] != 0x6E && buffer[0] != 0x6F {
        return false;
    }
    buffer
        .get(5..9)
        .map(|size| u32::from_le_bytes(size.try_into().unwrap()) == original_size)
        .unwrap_or(false)
}

fn paz_name(paz_num: u32) -> String {
    format!("pad{paz_num:05}.paz")
}

fn auto_rename_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();

    let mut index = 1u32;
    loop {
        let file_name = if extension.is_empty() {
            format!("{stem}[{index}]")
        } else {
            format!("{stem}[{index}].{extension}")
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
        index += 1;
    }
}

fn prompt_create_output_path(path: &Path) -> Result<OutputPathDecision> {
    eprintln!();
    eprintln!("Path \"{}\" doesn't exist.", path.display());
    eprintln!(">Create path? (Y)es / (n)o");
    loop {
        match read_prompt_char('y')? {
            'y' => return Ok(OutputPathDecision::Create),
            'n' | 'e' => return Ok(OutputPathDecision::Abort),
            _ => {}
        }
    }
}

fn prompt_create_path(path: &Path) -> Result<CreatePathDecision> {
    eprintln!();
    eprintln!("Path \"{}\" doesn't exist.", path.display());
    eprintln!(">Create path? (Y)es / (a)lways / (s)kip file / (e)xit");
    loop {
        match read_prompt_char('y')? {
            'y' => return Ok(CreatePathDecision::Yes),
            'a' => return Ok(CreatePathDecision::Always),
            's' => return Ok(CreatePathDecision::Skip),
            'e' => return Ok(CreatePathDecision::Abort),
            _ => {}
        }
    }
}

fn prompt_file_conflict(path: &Path) -> Result<FileConflictDecision> {
    eprintln!();
    eprintln!(
        "File \"{}\" already exists in target path.",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    );
    eprintln!("> (o)verwrite / (R)ename / overwrite (a)ll / re(n)ame all / (e)xit");
    loop {
        match read_prompt_char('r')? {
            'o' => return Ok(FileConflictDecision::Overwrite),
            'r' => return Ok(FileConflictDecision::Rename),
            'a' => return Ok(FileConflictDecision::OverwriteAll),
            'n' => return Ok(FileConflictDecision::RenameAll),
            'e' => return Ok(FileConflictDecision::Abort),
            _ => {}
        }
    }
}

fn read_prompt_char(default: char) -> Result<char> {
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let trimmed = line.trim();
    Ok(trimmed
        .chars()
        .next()
        .unwrap_or(default)
        .to_ascii_lowercase())
}
