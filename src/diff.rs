use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, File, Metadata};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

impl EntryKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
            Self::Symlink => "symlink",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryInfo {
    pub kind: EntryKind,
    pub len: u64,
    pub modified: Option<SystemTime>,
    pub symlink_target: Option<PathBuf>,
    platform_marker: PlatformMarker,
}

impl EntryInfo {
    pub fn description(&self) -> String {
        match self.kind {
            EntryKind::File => format!("file · {}", human_bytes(self.len)),
            EntryKind::Directory => "directory".to_owned(),
            EntryKind::Symlink => self
                .symlink_target
                .as_ref()
                .map(|target| format!("symlink → {}", target.display()))
                .unwrap_or_else(|| "symlink".to_owned()),
            EntryKind::Other => format!("other · {}", human_bytes(self.len)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    LeftOnly,
    RightOnly,
    Modified,
    TypeChanged,
    Identical,
}

impl DiffKind {
    pub fn plain_label(self) -> &'static str {
        match self {
            Self::LeftOnly => "← left",
            Self::RightOnly => "right →",
            Self::Modified => "≠ changed",
            Self::TypeChanged => "⇄ type",
            Self::Identical => "= same",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEntry {
    pub path: PathBuf,
    pub kind: DiffKind,
    pub left: Option<EntryInfo>,
    pub right: Option<EntryInfo>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffSummary {
    pub left_only: usize,
    pub right_only: usize,
    pub modified: usize,
    pub type_changed: usize,
    pub identical: usize,
}

impl DiffSummary {
    pub fn differences(&self) -> usize {
        self.left_only + self.right_only + self.modified + self.type_changed
    }

    pub fn total(&self) -> usize {
        self.differences() + self.identical
    }
}

#[derive(Debug, Clone)]
pub struct DiffReport {
    pub left_root: PathBuf,
    pub right_root: PathBuf,
    pub entries: Vec<DiffEntry>,
    pub summary: DiffSummary,
    pub scanned_at: SystemTime,
}

impl DiffReport {
    pub fn has_differences(&self) -> bool {
        self.summary.differences() > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileFingerprint {
    len: u64,
    modified_nanos: Option<u128>,
    platform_marker: PlatformMarker,
}

#[derive(Debug, Clone)]
struct CachedDigest {
    fingerprint: FileFingerprint,
    digest: blake3::Hash,
}

pub struct DiffEngine {
    left_root: PathBuf,
    right_root: PathBuf,
    digest_cache: HashMap<PathBuf, CachedDigest>,
}

impl DiffEngine {
    pub fn new(left: impl AsRef<Path>, right: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            left_root: canonical_directory(left.as_ref())?,
            right_root: canonical_directory(right.as_ref())?,
            digest_cache: HashMap::new(),
        })
    }

    pub fn scan(&mut self) -> Result<DiffReport> {
        let left_entries = scan_tree(&self.left_root)?;
        let right_entries = scan_tree(&self.right_root)?;
        let paths: BTreeSet<PathBuf> = left_entries
            .keys()
            .chain(right_entries.keys())
            .cloned()
            .collect();

        let mut entries = Vec::with_capacity(paths.len());
        let mut summary = DiffSummary::default();

        for path in paths {
            let left = left_entries.get(&path);
            let right = right_entries.get(&path);
            let kind = match (left, right) {
                (Some(_), None) => DiffKind::LeftOnly,
                (None, Some(_)) => DiffKind::RightOnly,
                (Some(left), Some(right)) => self
                    .compare_pair(&path, left, right)
                    .with_context(|| format!("failed to compare {}", path.display()))?,
                (None, None) => unreachable!("union path must exist on at least one side"),
            };

            match kind {
                DiffKind::LeftOnly => summary.left_only += 1,
                DiffKind::RightOnly => summary.right_only += 1,
                DiffKind::Modified => summary.modified += 1,
                DiffKind::TypeChanged => summary.type_changed += 1,
                DiffKind::Identical => summary.identical += 1,
            }

            entries.push(DiffEntry {
                path,
                kind,
                left: left.cloned(),
                right: right.cloned(),
            });
        }

        Ok(DiffReport {
            left_root: self.left_root.clone(),
            right_root: self.right_root.clone(),
            entries,
            summary,
            scanned_at: SystemTime::now(),
        })
    }

    fn compare_pair(
        &mut self,
        relative: &Path,
        left: &EntryInfo,
        right: &EntryInfo,
    ) -> Result<DiffKind> {
        if left.kind != right.kind {
            return Ok(DiffKind::TypeChanged);
        }

        let identical = match left.kind {
            EntryKind::Directory => true,
            EntryKind::Symlink => left.symlink_target == right.symlink_target,
            EntryKind::Other => left.len == right.len && left.modified == right.modified,
            EntryKind::File => {
                if left.len != right.len {
                    false
                } else {
                    let left_path = self.left_root.join(relative);
                    let right_path = self.right_root.join(relative);
                    self.digest(&left_path, left)? == self.digest(&right_path, right)?
                }
            }
        };

        Ok(if identical {
            DiffKind::Identical
        } else {
            DiffKind::Modified
        })
    }

    fn digest(&mut self, path: &Path, info: &EntryInfo) -> Result<blake3::Hash> {
        let fingerprint = FileFingerprint {
            len: info.len,
            modified_nanos: info
                .modified
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos()),
            platform_marker: info.platform_marker,
        };
        if let Some(cached) = self.digest_cache.get(path)
            && cached.fingerprint == fingerprint
        {
            return Ok(cached.digest);
        }

        let mut file =
            File::open(path).with_context(|| format!("failed to open file {}", path.display()))?;
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = file
                .read(&mut buffer)
                .with_context(|| format!("failed to read file {}", path.display()))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        let digest = hasher.finalize();
        self.digest_cache.insert(
            path.to_path_buf(),
            CachedDigest {
                fingerprint,
                digest,
            },
        );
        Ok(digest)
    }
}

fn canonical_directory(path: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("directory does not exist: {}", path.display()))?;
    if !canonical.is_dir() {
        bail!("path is not a directory: {}", path.display());
    }
    Ok(canonical)
}

fn scan_tree(root: &Path) -> Result<BTreeMap<PathBuf, EntryInfo>> {
    let mut entries = BTreeMap::new();
    for item in WalkDir::new(root).follow_links(false) {
        let item = item.with_context(|| format!("failed to walk {}", root.display()))?;
        if item.depth() == 0 {
            continue;
        }
        let relative = item
            .path()
            .strip_prefix(root)
            .expect("walked path must be below its root")
            .to_path_buf();
        let metadata = fs::symlink_metadata(item.path())
            .with_context(|| format!("failed to inspect {}", item.path().display()))?;
        entries.insert(relative, entry_info(item.path(), &metadata)?);
    }
    Ok(entries)
}

fn entry_info(path: &Path, metadata: &Metadata) -> Result<EntryInfo> {
    let file_type = metadata.file_type();
    let kind = if file_type.is_file() {
        EntryKind::File
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else if file_type.is_symlink() {
        EntryKind::Symlink
    } else {
        EntryKind::Other
    };

    Ok(EntryInfo {
        kind,
        len: metadata.len(),
        modified: metadata.modified().ok(),
        symlink_target: if kind == EntryKind::Symlink {
            Some(
                fs::read_link(path)
                    .with_context(|| format!("failed to read symlink {}", path.display()))?,
            )
        } else {
            None
        },
        platform_marker: platform_marker(metadata),
    })
}

#[cfg(unix)]
type PlatformMarker = (u64, u64, i64, i64);

#[cfg(not(unix))]
type PlatformMarker = ();

#[cfg(unix)]
fn platform_marker(metadata: &Metadata) -> PlatformMarker {
    use std::os::unix::fs::MetadataExt;

    (
        metadata.dev(),
        metadata.ino(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

#[cfg(not(unix))]
fn platform_marker(_metadata: &Metadata) -> PlatformMarker {}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn scan_classifies_folder_differences() {
        let left = tempdir().unwrap();
        let right = tempdir().unwrap();

        fs::write(left.path().join("same.txt"), "same").unwrap();
        fs::write(right.path().join("same.txt"), "same").unwrap();
        fs::write(left.path().join("changed.txt"), "left").unwrap();
        fs::write(right.path().join("changed.txt"), "rght").unwrap();
        fs::write(left.path().join("left.txt"), "left").unwrap();
        fs::write(right.path().join("right.txt"), "right").unwrap();
        fs::create_dir(left.path().join("kind")).unwrap();
        fs::write(right.path().join("kind"), "file").unwrap();

        let report = DiffEngine::new(left.path(), right.path())
            .unwrap()
            .scan()
            .unwrap();

        assert_eq!(report.summary.left_only, 1);
        assert_eq!(report.summary.right_only, 1);
        assert_eq!(report.summary.modified, 1);
        assert_eq!(report.summary.type_changed, 1);
        assert_eq!(report.summary.identical, 1);
        assert_eq!(report.summary.differences(), 4);
    }

    #[test]
    fn same_sized_files_are_compared_by_content() {
        let left = tempdir().unwrap();
        let right = tempdir().unwrap();
        fs::write(left.path().join("value"), "abc").unwrap();
        fs::write(right.path().join("value"), "xyz").unwrap();

        let report = DiffEngine::new(left.path(), right.path())
            .unwrap()
            .scan()
            .unwrap();

        assert_eq!(report.summary.modified, 1);
        assert_eq!(report.entries[0].kind, DiffKind::Modified);
    }

    #[test]
    fn digest_cache_is_invalidated_when_a_path_is_replaced() {
        let left = tempdir().unwrap();
        let right = tempdir().unwrap();
        fs::write(left.path().join("value"), "abc").unwrap();
        fs::write(right.path().join("value"), "abc").unwrap();
        let mut engine = DiffEngine::new(left.path(), right.path()).unwrap();

        assert!(!engine.scan().unwrap().has_differences());
        fs::remove_file(right.path().join("value")).unwrap();
        fs::write(right.path().join("value"), "xyz").unwrap();

        let report = engine.scan().unwrap();
        assert_eq!(report.summary.modified, 1);
    }

    #[test]
    fn rejects_non_directory_roots() {
        let root = tempdir().unwrap();
        let file = root.path().join("file");
        fs::write(&file, "value").unwrap();

        let error = DiffEngine::new(&file, root.path())
            .err()
            .expect("file root should fail");
        assert!(error.to_string().contains("not a directory"));
    }

    #[test]
    fn formats_human_readable_sizes() {
        assert_eq!(human_bytes(42), "42 B");
        assert_eq!(human_bytes(1024), "1.0 KiB");
        assert_eq!(human_bytes(1_572_864), "1.5 MiB");
    }

    #[test]
    fn bundled_demo_corpus_keeps_its_documented_classifications() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let report = DiffEngine::new(
            root.join("data/folder1/demo"),
            root.join("data/folder2/demo"),
        )
        .unwrap()
        .scan()
        .unwrap();

        assert_eq!(
            report.summary,
            DiffSummary {
                left_only: 3,
                right_only: 2,
                modified: 8,
                type_changed: 1,
                identical: 5,
            }
        );
    }
}
