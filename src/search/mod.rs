//! Full-text search over the document directory. [`update_index`] builds or
//! updates its cache under the user's settings directory. [`search`] queries it.
//!
//! Each document directory gets its own subdirectory under `.search-index/`.
//! Inside it, Tantivy uses `tantivy/`. The sibling file `tantivy.meta`
//! records the schema version on its first line, then one
//! "<mtime>\t<relative path>" line per indexed file. That file is what makes
//! [`update_index`] incremental.

pub mod analyzer;
pub mod extract;

use crate::paths;
use crate::pkm::{self, INDEX_DIR_NAME};
use analyzer::{CONTENT_TOKENIZER, FILENAME_TOKENIZER, JapaneseTokenizer};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File, TryLockError};
use std::io::ErrorKind;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::{BooleanQuery, BoostQuery, Occur, PhraseQuery, Query, TermQuery};
use tantivy::schema::{FAST, Field, IndexRecordOption, STORED, STRING, Schema, TextFieldIndexing, TextOptions, Value};
use tantivy::snippet::SnippetGenerator;
use tantivy::{DocAddress, Index, IndexReader, Order, ReloadPolicy, TantivyDocument, Term, doc};

/// Bump whenever the analyzer or the indexed fields change so that old and new
/// tokens no longer match; update_index() then rebuilds the index from scratch.
// Version 2 also discards content previously indexed through links.
pub const INDEX_SCHEMA_VERSION: u32 = 2;

const TANTIVY_SUBDIR_NAME: &str = "tantivy";
const META_FILE_NAME: &str = "tantivy.meta";
const META_VERSION_PREFIX: &str = "version=";
const OWNER_FILE_NAME: &str = ".mind-pkm-index";
const OWNER_CONTENT: &str = "Mind PKM Tantivy cache\n";
const UPDATE_LOCK_NAME: &str = "update.lock";

// As far into the content as a snippet is looked for.
const PREVIEW_CHARS: usize = 50 * 1024;
const SNIPPET_CHARS: usize = 160;
pub const DEFAULT_MAX_RESULTS: usize = 100;
// A file whose name matches ranks above one that merely mentions the keyword.
const FILENAME_BOOST: f32 = 2.0;
const CONTENT_BOOST: f32 = 1.0;
const WRITER_MEMORY: usize = 50_000_000;

// The fields of the index.
const PATH_FIELD: &str = "path";
const FILENAME_FIELD: &str = "filename";
const CONTENT_FIELD: &str = "content";
const PREVIEW_FIELD: &str = "preview";
const KIND_FIELD: &str = "kind";
const MODIFIED_FIELD: &str = "modified";

/// One search result: the matched file, its path relative to the document
/// directory ("/"-separated), its mtime (ms) when it was indexed, and a content
/// snippet around the match (empty when the file only matched by name).
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub file: PathBuf,
    pub relative_path: String,
    pub last_modified: i64,
    pub snippet: Snippet,
}

/// A piece of text with the byte ranges of the matched terms.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snippet {
    pub text: String,
    pub highlights: Vec<Range<usize>>,
}

/// Result order: newest mtime first, or best match first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchSort {
    Modified,
    Relevance,
}

/// Which files a search returns; see `pkm::is_page_path` for what counts as a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchScope {
    Pages,
    Assets,
    All,
}

impl SearchScope {
    fn kind(self) -> Option<&'static str> {
        match self {
            SearchScope::Pages => Some("page"),
            SearchScope::Assets => Some("asset"),
            SearchScope::All => None,
        }
    }

    fn of(rel_path: &str) -> SearchScope {
        if pkm::is_page_path(rel_path) { SearchScope::Pages } else { SearchScope::Assets }
    }
}

struct Fields {
    path: Field,
    filename: Field,
    content: Field,
    preview: Field,
    kind: Field,
    modified: Field,
}

fn schema() -> Schema {
    let mut b = Schema::builder();
    let text = |tokenizer: &str| {
        TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer(tokenizer)
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        )
    };
    b.add_text_field(PATH_FIELD, STRING | STORED);
    b.add_text_field(FILENAME_FIELD, text(FILENAME_TOKENIZER).set_stored());
    b.add_text_field(CONTENT_FIELD, text(CONTENT_TOKENIZER));
    b.add_text_field(PREVIEW_FIELD, STORED);
    b.add_text_field(KIND_FIELD, STRING);
    b.add_i64_field(MODIFIED_FIELD, FAST | STORED);
    b.build()
}

fn fields_of(index: &Index) -> Result<Fields, String> {
    let s = index.schema();
    let f = |name: &str| s.get_field(name).map_err(|e| e.to_string());
    Ok(Fields {
        path: f(PATH_FIELD)?,
        filename: f(FILENAME_FIELD)?,
        content: f(CONTENT_FIELD)?,
        preview: f(PREVIEW_FIELD)?,
        kind: f(KIND_FIELD)?,
        modified: f(MODIFIED_FIELD)?,
    })
}

fn register_tokenizers(index: &Index) {
    index.tokenizers().register(CONTENT_TOKENIZER, JapaneseTokenizer::content());
    index.tokenizers().register(FILENAME_TOKENIZER, JapaneseTokenizer::filename());
}

/// A stable, fixed-length filename derived from the canonical OS path.
fn cache_dir(config_dir: &Path, doc_dir: &Path) -> PathBuf {
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(unix)]
    let path_bytes = doc_dir.as_os_str().as_bytes();
    #[cfg(windows)]
    use std::os::windows::ffi::OsStrExt;
    #[cfg(windows)]
    let path_bytes: Vec<u8> = doc_dir.as_os_str().encode_wide().flat_map(u16::to_le_bytes).collect();
    let id = Sha256::digest(path_bytes);
    config_dir.join(INDEX_DIR_NAME).join(format!("{id:x}"))
}

/// Check both the cache's ancestors and its contents before Tantivy gets any
/// paths. In particular, checking only `tantivy/` misses a linked cache root,
/// metadata file, segment, or lock file.
fn checked_cache(config_dir: &Path, cache_dir: &Path) -> Result<(PathBuf, PathBuf), String> {
    fn check_tree(path: &Path, root: &Path) -> Result<(), String> {
        let path = paths::resolve_inside(path, root).map_err(|e| e.to_string())?;
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.to_string()),
        };
        if paths::is_link(&metadata) {
            return Err("search cache contains a link".into());
        }
        if metadata.is_dir() {
            for entry in std::fs::read_dir(path).map_err(|e| e.to_string())? {
                check_tree(&entry.map_err(|e| e.to_string())?.path(), root)?;
            }
        } else if !metadata.is_file() {
            return Err("search cache contains a non-regular file".into());
        }
        Ok(())
    }
    let resolve = |name| paths::resolve_inside(&cache_dir.join(name), config_dir).map_err(|e| e.to_string());
    let dir = resolve(TANTIVY_SUBDIR_NAME)?;
    let meta = resolve(META_FILE_NAME)?;
    check_tree(&dir, config_dir)?;
    if meta.exists() && !meta.is_file() {
        return Err("search metadata is not a regular file".into());
    }
    Ok((dir, meta))
}

/// Hold the lock outside `tantivy/` so it spans rebuilds and metadata writes.
fn lock_cache(config_dir: &Path, cache_dir: &Path) -> Result<Option<File>, String> {
    let path = paths::resolve_inside(&cache_dir.join(UPDATE_LOCK_NAME), config_dir).map_err(|e| e.to_string())?;
    if path.exists() && !path.is_file() {
        return Err("search cache lock is not a regular file".into());
    }
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let file =
        File::options().read(true).write(true).create(true).truncate(false).open(path).map_err(|e| e.to_string())?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(e)) => Err(e.to_string()),
    }
}

/// A fixed cache name is not evidence of ownership. Old app indexes can be
/// rebuilt when their schema matches. Keep an unrelated nonempty folder.
fn owns_cache(dir: &Path) -> bool {
    if std::fs::read_to_string(dir.join(OWNER_FILE_NAME)).is_ok_and(|s| s == OWNER_CONTENT) {
        return true;
    }
    Index::open_in_dir(dir).is_ok_and(|index| index.schema() == schema())
}

fn index_exists(dir: &Path) -> bool {
    dir.is_dir() && MmapDirectory::open(dir).ok().is_some_and(|d| Index::exists(&d).unwrap_or(false))
}

struct Meta {
    version: Option<u32>,
    mtimes: HashMap<String, i64>,
}

fn load_meta(path: &Path) -> Meta {
    let mut meta = Meta { version: None, mtimes: HashMap::new() };
    let Ok(text) = std::fs::read_to_string(path) else { return meta };
    let mut lines = text.lines();
    let Some(version) = lines.next().and_then(|l| l.strip_prefix(META_VERSION_PREFIX)).and_then(|v| v.parse().ok())
    else {
        return meta;
    };
    meta.version = Some(version);
    for line in lines {
        if let Some((mtime, rel)) = line.split_once('\t')
            && let Ok(m) = mtime.parse()
            && !rel.is_empty()
        {
            meta.mtimes.insert(rel.to_string(), m);
        }
    }
    meta
}

fn save_meta(path: &Path, mtimes: &HashMap<String, i64>) -> std::io::Result<()> {
    let mut out = format!("{META_VERSION_PREFIX}{INDEX_SCHEMA_VERSION}\n");
    let mut entries: Vec<_> = mtimes.iter().collect();
    entries.sort();
    for (rel, mtime) in entries {
        out.push_str(&format!("{mtime}\t{rel}\n"));
    }
    // Replace the directory entry instead of truncating a possibly substituted
    // symlink/hardlink. The temporary file is created exclusively by tempfile.
    use std::io::Write;
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    file.write_all(out.as_bytes())?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

fn mtime_millis(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Every regular file under doc_dir with its relative path, skipping
/// dot-directories (the index itself, .git, ...) and all links/reparse points.
fn indexable_files(doc_dir: &Path) -> Vec<(PathBuf, String)> {
    fn walk(dir: &Path, root: &Path, rel: &str, out: &mut Vec<(PathBuf, String)>) {
        if dir != root && paths::resolve_inside(dir, root).is_err() {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            let rel_path = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            let Ok(metadata) = std::fs::symlink_metadata(&path) else { continue };
            if paths::is_link(&metadata) || paths::resolve_inside(&path, root).is_err() {
                continue;
            }
            if metadata.is_dir() {
                if !name.starts_with('.') {
                    walk(&path, root, &rel_path, out);
                }
            } else if metadata.is_file() {
                out.push((path, rel_path));
            }
        }
    }
    let mut out = Vec::new();
    walk(doc_dir, doc_dir, "", &mut out);
    out
}

// One update at a time per process; another process holding the index lock
// makes the update a no-op.
static UPDATE_LOCK: Mutex<()> = Mutex::new(());

/// Incrementally updates the index: new/modified files (per the recorded
/// mtime) are (re-)extracted, deleted files are dropped, unmodified files are
/// left alone. Rebuilds everything when the recorded schema version is stale or
/// the index itself is missing. The file walk and the diff come first, so the
/// common "nothing changed" run never opens the index for writing.
pub fn update_index(doc_dir: &Path) -> Result<(), String> {
    let config_dir = paths::user_config_dir().ok_or("user settings directory is unavailable")?;
    update_index_in(&config_dir, doc_dir)
}

pub(crate) fn update_index_in(config_dir: &Path, doc_dir: &Path) -> Result<(), String> {
    let _guard = UPDATE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = doc_dir.canonicalize().map_err(|e| e.to_string())?;
    let doc_dir = root.as_path();
    if !doc_dir.is_dir() {
        return Err("search document directory is not a directory".into());
    }
    fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let config_dir = config_dir.canonicalize().map_err(|e| e.to_string())?;
    let cache_dir = cache_dir(&config_dir, doc_dir);
    let Some(_cache_lock) = lock_cache(&config_dir, &cache_dir)? else { return Ok(()) };
    let (dir, meta_path) = checked_cache(&config_dir, &cache_dir)?;
    let meta = load_meta(&meta_path);
    let rebuild = meta.version != Some(INDEX_SCHEMA_VERSION) || !index_exists(&dir);
    let known = if rebuild { HashMap::new() } else { meta.mtimes };

    let mut current = HashMap::new();
    let mut changed = Vec::new();
    for (path, rel) in indexable_files(doc_dir) {
        let mtime = mtime_millis(&path);
        if known.get(&rel) != Some(&mtime) {
            changed.push((path, rel.clone()));
        }
        current.insert(rel, mtime);
    }
    let deleted: Vec<&String> = known.keys().filter(|k| !current.contains_key(*k)).collect();
    // Recheck after the potentially long document walk, including a no-op run.
    checked_cache(&config_dir, &cache_dir)?;
    if !rebuild && changed.is_empty() && deleted.is_empty() {
        return Ok(());
    }

    let index = if rebuild {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let nonempty =
            fs::read_dir(&dir).map_err(|e| e.to_string())?.next().transpose().map_err(|e| e.to_string())?.is_some();
        if nonempty && !owns_cache(&dir) {
            return Err("refusing to replace a search cache not owned by Mind PKM".into());
        }
        checked_cache(&config_dir, &cache_dir)?;
        // An interrupted rebuild must leave no metadata that validates the new index.
        match fs::remove_file(&meta_path) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
        if nonempty {
            fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        }
        fs::write(dir.join(OWNER_FILE_NAME), OWNER_CONTENT).map_err(|e| e.to_string())?;
        Index::create_in_dir(&dir, schema()).map_err(|e| e.to_string())?
    } else {
        Index::open_in_dir(&dir).map_err(|e| e.to_string())?
    };
    register_tokenizers(&index);
    let f = fields_of(&index)?;
    let mut writer = index.writer::<TantivyDocument>(WRITER_MEMORY).map_err(|e| e.to_string())?;
    for (path, rel) in &changed {
        writer.delete_term(Term::from_field_text(f.path, rel));
        // A file/parent may have changed since enumeration; do not use it then.
        let Ok(safe_path) = paths::resolve_inside(path, doc_dir) else {
            current.remove(rel);
            continue;
        };
        let text = extract::extract_text(&safe_path);
        let preview = extract::truncate_chars(text.clone(), PREVIEW_CHARS);
        let filename = crate::paths::file_name(path);
        writer
            .add_document(doc!(
                f.path => rel.as_str(),
                f.filename => filename,
                f.content => text,
                f.preview => preview,
                f.kind => SearchScope::of(rel).kind().unwrap(),
                f.modified => current[rel],
            ))
            .map_err(|e| e.to_string())?;
    }
    for rel in deleted {
        writer.delete_term(Term::from_field_text(f.path, rel));
    }
    writer.commit().map_err(|e| e.to_string())?;
    writer.wait_merging_threads().map_err(|e| e.to_string())?;
    // After the commit only, so an interrupted update can retry.
    checked_cache(&config_dir, &cache_dir)?;
    save_meta(&meta_path, &current).map_err(|e| e.to_string())
}

/// The query for one keyword in one field: its tokens as a phrase (a term for
/// a single token). None when the keyword has no token.
fn field_query(field: Field, keyword: &str, split_words: bool, boost: f32) -> Option<Box<dyn Query>> {
    let terms: Vec<Term> =
        analyzer::analyze(keyword, split_words).into_iter().map(|t| Term::from_field_text(field, &t.text)).collect();
    let q: Box<dyn Query> = match terms.len() {
        0 => return None,
        1 => Box::new(TermQuery::new(terms.into_iter().next().unwrap(), IndexRecordOption::WithFreqs)),
        _ => Box::new(PhraseQuery::new(terms)),
    };
    Some(Box::new(BoostQuery::new(q, boost)))
}

/// Every keyword must match (AND); a keyword matches in either the file name
/// or the content (OR, the name boosted). Keywords are literal text, never
/// query syntax.
fn keyword_query(f: &Fields, keywords: &[&str]) -> Option<BooleanQuery> {
    let mut clauses: Vec<(Occur, Box<dyn Query>)> = Vec::new();
    for k in keywords {
        let per_field: Vec<(Occur, Box<dyn Query>)> =
            [field_query(f.filename, k, true, FILENAME_BOOST), field_query(f.content, k, false, CONTENT_BOOST)]
                .into_iter()
                .flatten()
                .map(|q| (Occur::Should, q))
                .collect();
        if !per_field.is_empty() {
            clauses.push((Occur::Must, Box::new(BooleanQuery::new(per_field))));
        }
    }
    (!clauses.is_empty()).then(|| BooleanQuery::new(clauses))
}

fn open_reader(index: &Index) -> Result<IndexReader, String> {
    index.reader_builder().reload_policy(ReloadPolicy::Manual).try_into().map_err(|e| e.to_string())
}

/// Files matching every whitespace-separated keyword (a full-width space
/// separates keywords too) in content or file name, restricted to `scope` and
/// ordered by `sort`. Empty when the index doesn't exist yet or no keyword
/// tokenizes to anything.
pub fn search(
    doc_dir: &Path,
    query_text: &str,
    sort: SearchSort,
    scope: SearchScope,
    max_results: usize,
) -> Result<Vec<SearchHit>, String> {
    let config_dir = paths::user_config_dir().ok_or("user settings directory is unavailable")?;
    search_in(&config_dir, doc_dir, query_text, sort, scope, max_results)
}

fn search_in(
    config_dir: &Path,
    doc_dir: &Path,
    query_text: &str,
    sort: SearchSort,
    scope: SearchScope,
    max_results: usize,
) -> Result<Vec<SearchHit>, String> {
    let keywords: Vec<&str> =
        query_text.split(|c: char| c.is_whitespace() || c == '\u{3000}').filter(|k| !k.is_empty()).collect();
    let root = doc_dir.canonicalize().map_err(|e| e.to_string())?;
    if !root.is_dir() {
        return Err("search document directory is not a directory".into());
    }
    let config_dir = match config_dir.canonicalize() {
        Ok(path) => path,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.to_string()),
    };
    let (dir, meta_path) = checked_cache(&config_dir, &cache_dir(&config_dir, &root))?;
    if keywords.is_empty() || load_meta(&meta_path).version != Some(INDEX_SCHEMA_VERSION) || !index_exists(&dir) {
        return Ok(Vec::new());
    }
    let index = Index::open_in_dir(&dir).map_err(|e| e.to_string())?;
    register_tokenizers(&index);
    let f = fields_of(&index)?;
    let Some(kq) = keyword_query(&f, &keywords) else { return Ok(Vec::new()) };
    let query: Box<dyn Query> = match scope.kind() {
        None => Box::new(kq.clone()),
        Some(kind) => {
            let filter = TermQuery::new(Term::from_field_text(f.kind, kind), IndexRecordOption::Basic);
            Box::new(BooleanQuery::new(vec![
                (Occur::Must, Box::new(kq.clone()) as Box<dyn Query>),
                (Occur::Must, Box::new(BoostQuery::new(Box::new(filter), 0.0))),
            ]))
        }
    };

    let searcher = open_reader(&index)?.searcher();
    let limit = TopDocs::with_limit(max_results.max(1));
    let addresses = match sort {
        SearchSort::Modified => {
            searcher.search(&*query, &limit.order_by_fast_field::<i64>(MODIFIED_FIELD, Order::Desc)).map(addresses)
        }
        SearchSort::Relevance => searcher.search(&*query, &limit.order_by_score()).map(addresses),
    }
    .map_err(|e| e.to_string())?;

    let mut generator = SnippetGenerator::create(&searcher, &kq, f.content).map_err(|e| e.to_string())?;
    generator.set_max_num_chars(SNIPPET_CHARS);
    let mut hits = Vec::new();
    for addr in addresses {
        let d: TantivyDocument = searcher.doc(addr).map_err(|e| e.to_string())?;
        let rel = d.get_first(f.path).and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let file = doc_dir.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        if paths::resolve_inside(&file, doc_dir).is_err() {
            continue;
        }
        let preview = d.get_first(f.preview).and_then(|v| v.as_str()).unwrap_or_default();
        let modified = d.get_first(f.modified).and_then(|v| v.as_i64()).unwrap_or(0);
        hits.push(SearchHit {
            file,
            relative_path: rel,
            last_modified: modified,
            snippet: best_snippet(&generator, preview),
        });
    }
    Ok(hits)
}

/// The documents of a ranked result, without what they were ranked by.
fn addresses<T>(ranked: Vec<(T, DocAddress)>) -> Vec<DocAddress> {
    ranked.into_iter().map(|(_, a)| a).collect()
}

fn best_snippet(generator: &SnippetGenerator, preview: &str) -> Snippet {
    if preview.is_empty() {
        return Snippet::default();
    }
    let s = generator.snippet(preview);
    if !s.fragment().trim().is_empty() && !s.highlighted().is_empty() {
        let fragment = s.fragment();
        let lead = fragment.len() - fragment.trim_start().len();
        let text = fragment.trim().to_string();
        let highlights = s
            .highlighted()
            .iter()
            .map(|r| r.start.saturating_sub(lead)..(r.end.saturating_sub(lead)).min(text.len()))
            .filter(|r| r.start < r.end)
            .collect();
        return Snippet { text, highlights };
    }
    let mut text = extract::truncate_chars(preview.to_string(), SNIPPET_CHARS);
    if text.len() < preview.len() {
        text.push('…');
    }
    Snippet { text: text.trim().to_string(), highlights: Vec::new() }
}

/// Snippet text with newlines and runs of whitespace collapsed, highlights kept.
pub fn single_line(snippet: &Snippet) -> Snippet {
    let mut text = String::new();
    let mut map = Vec::with_capacity(snippet.text.len() + 1);
    let mut last_space = false;
    for c in snippet.text.chars() {
        let ws = c.is_whitespace();
        for _ in 0..c.len_utf8() {
            map.push(text.len());
        }
        if ws {
            if !last_space {
                text.push(' ');
            }
        } else {
            text.push(c);
        }
        last_space = ws;
    }
    map.push(text.len());
    let highlights = snippet
        .highlights
        .iter()
        .map(|r| map[r.start]..map[r.end.min(map.len() - 1)])
        .filter(|r| r.start < r.end)
        .collect();
    Snippet { text, highlights }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, rel: &str, text: &str) -> PathBuf {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, text).unwrap();
        p
    }

    fn paths(hits: &[SearchHit]) -> Vec<&str> {
        let mut v: Vec<&str> = hits.iter().map(|h| h.relative_path.as_str()).collect();
        v.sort();
        v
    }

    fn test_cache(config: &Path, dir: &Path) -> PathBuf {
        cache_dir(config, &dir.canonicalize().unwrap())
    }

    fn find(config: &Path, dir: &Path, q: &str, scope: SearchScope) -> Vec<SearchHit> {
        search_in(config, dir, q, SearchSort::Relevance, scope, DEFAULT_MAX_RESULTS).unwrap()
    }

    #[test]
    fn indexes_and_searches_incrementally() {
        let tmp = tempfile::tempdir().unwrap();
        let (d, config) = (&tmp.path().join("docs"), &tmp.path().join("config/mind-pkm"));
        write(d, "Rust入門.md", "\u{feff}# メモ\n\n東京都の関西国際空港について書いた本を読んだ。");
        write(d, "Rust入門.assets/notes.txt", "attachment about tokyo");
        write(d, "QuarterlyReport2024.txt", "numbers");
        write(d, ".git/config", "secret tokyo");
        assert!(find(config, d, "anything", SearchScope::All).is_empty());
        assert!(!config.exists());
        update_index_in(config, d).unwrap();
        assert!(!d.join(INDEX_DIR_NAME).exists());

        assert_eq!(paths(&find(config, d, "空港", SearchScope::All)), vec!["Rust入門.md"]);
        assert!(find(config, d, "京都", SearchScope::All).is_empty());
        assert_eq!(paths(&find(config, d, "読む", SearchScope::All)), vec!["Rust入門.md"]);
        // AND across keywords, full-width space too.
        assert_eq!(paths(&find(config, d, "東京　空港", SearchScope::All)), vec!["Rust入門.md"]);
        assert!(find(config, d, "空港 numbers", SearchScope::All).is_empty());
        // File names, split into word parts.
        assert_eq!(paths(&find(config, d, "report", SearchScope::All)), vec!["QuarterlyReport2024.txt"]);
        assert_eq!(paths(&find(config, d, "rust", SearchScope::All)), vec!["Rust入門.md"]);
        // Scopes.
        assert_eq!(paths(&find(config, d, "tokyo", SearchScope::All)), vec!["Rust入門.assets/notes.txt"]);
        assert!(find(config, d, "tokyo", SearchScope::Pages).is_empty());
        assert_eq!(paths(&find(config, d, "tokyo", SearchScope::Assets)), vec!["Rust入門.assets/notes.txt"]);

        // Snippet with highlights.
        let hit = &find(config, d, "空港", SearchScope::All)[0];
        let s = &hit.snippet;
        assert!(!s.highlights.is_empty(), "{s:?}");
        assert!(s.highlights.iter().all(|r| s.text[r.clone()].contains("空港")), "{s:?}");

        // Incremental: a deleted file disappears, a new one appears.
        std::fs::remove_file(d.join("QuarterlyReport2024.txt")).unwrap();
        write(d, "sub/new page.md", "numbers again");
        update_index_in(config, d).unwrap();
        assert_eq!(paths(&find(config, d, "numbers", SearchScope::All)), vec!["sub/new page.md"]);
        let meta = fs::read_to_string(test_cache(config, d).join(META_FILE_NAME)).unwrap();
        assert!(meta.starts_with(&format!("version={INDEX_SCHEMA_VERSION}\n")));
        assert!(!meta.contains(".git"));
    }

    #[test]
    fn rebuilds_when_index_is_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let (d, config) = (&tmp.path().join("docs"), &tmp.path().join("config/mind-pkm"));
        write(d, "a.md", "hello world");
        update_index_in(config, d).unwrap();
        fs::remove_dir_all(test_cache(config, d).join(TANTIVY_SUBDIR_NAME)).unwrap();
        update_index_in(config, d).unwrap();
        assert_eq!(paths(&find(config, d, "hello", SearchScope::All)), vec!["a.md"]);
    }

    #[test]
    fn separates_document_directories_with_the_same_name() {
        let tmp = tempfile::tempdir().unwrap();
        let config = tmp.path().join("config/mind-pkm");
        let first = tmp.path().join("first/docs");
        let second = tmp.path().join("second/docs");
        write(&first, "Page.md", "apple");
        write(&second, "Page.md", "banana");
        update_index_in(&config, &first).unwrap();
        update_index_in(&config, &second).unwrap();

        assert_eq!(paths(&find(&config, &first, "apple", SearchScope::All)), vec!["Page.md"]);
        assert!(find(&config, &first, "banana", SearchScope::All).is_empty());
        assert_eq!(paths(&find(&config, &second, "banana", SearchScope::All)), vec!["Page.md"]);
        assert!(find(&config, &second, "apple", SearchScope::All).is_empty());
        let caches = fs::read_dir(config.join(INDEX_DIR_NAME)).unwrap().count();
        assert_eq!(caches, 2);
    }

    #[test]
    fn builds_from_documents_and_preserves_old_tantivy_and_lucene_files() {
        let tmp = tempfile::tempdir().unwrap();
        let d = tmp.path().join("docs");
        let config = tmp.path().join("config/mind-pkm");
        let previous_config = tmp.path().join("previous");
        let page = write(&d, "Page.md", "original");
        update_index_in(&previous_config, &d).unwrap();
        let previous_cache = test_cache(&previous_config, &d);
        let old_path = d.join(INDEX_DIR_NAME);
        let lucene = write(&old_path, "lucene/segments", "lucene");
        let lucene_meta = write(&old_path, "files.meta", "freeplane");
        for name in [TANTIVY_SUBDIR_NAME, META_FILE_NAME] {
            fs::rename(previous_cache.join(name), old_path.join(name)).unwrap();
        }
        let old_meta = old_path.join(META_FILE_NAME);
        let old_index = old_path.join(TANTIVY_SUBDIR_NAME).join("meta.json");
        let old_meta_contents = fs::read(&old_meta).unwrap();
        let old_index_contents = fs::read(&old_index).unwrap();
        fs::write(page, "revised").unwrap();

        assert!(find(&config, &d, "original", SearchScope::All).is_empty());
        update_index_in(&config, &d).unwrap();
        assert!(find(&config, &d, "original", SearchScope::All).is_empty());
        assert_eq!(paths(&find(&config, &d, "revised", SearchScope::All)), vec!["Page.md"]);
        assert_eq!(fs::read(old_meta).unwrap(), old_meta_contents);
        assert_eq!(fs::read(old_index).unwrap(), old_index_contents);
        assert_eq!(fs::read_to_string(lucene).unwrap(), "lucene");
        assert_eq!(fs::read_to_string(lucene_meta).unwrap(), "freeplane");
    }

    #[test]
    fn leaves_an_active_update_alone_and_retries_after_the_lock_is_released() {
        let tmp = tempfile::tempdir().unwrap();
        let (d, config) = (&tmp.path().join("docs"), &tmp.path().join("config/mind-pkm"));
        write(d, "a.md", "hello");
        fs::create_dir_all(config).unwrap();
        let cache = test_cache(config, d);
        let lock = lock_cache(config, &cache).unwrap().unwrap();
        update_index_in(config, d).unwrap();
        assert!(!cache.join(TANTIVY_SUBDIR_NAME).exists());
        drop(lock);
        update_index_in(config, d).unwrap();
        assert_eq!(paths(&find(config, d, "hello", SearchScope::All)), vec!["a.md"]);
    }

    #[cfg(unix)]
    #[test]
    fn does_not_index_links_or_keep_results_after_a_file_becomes_a_link() {
        use std::os::unix::fs::symlink;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("docs");
        let config = tmp.path().join("config/mind-pkm");
        let outside = tmp.path().join("outside");
        let secret = write(&outside, "secret.txt", "confidential");
        let page = write(&root, "page.md", "ordinary");
        symlink(&outside, root.join("External.assets")).unwrap();
        symlink(&secret, root.join("external.txt")).unwrap();
        symlink(outside.join("missing"), root.join("dangling.txt")).unwrap();
        symlink(&root, root.join("cycle")).unwrap();
        symlink(&outside, root.join(INDEX_DIR_NAME)).unwrap();
        update_index_in(&config, &root).unwrap();
        let alias = tmp.path().join("alias");
        symlink(&root, &alias).unwrap();
        assert_eq!(paths(&find(&config, &alias, "ordinary", SearchScope::All)), vec!["page.md"]);
        assert!(find(&config, &root, "confidential", SearchScope::All).is_empty());
        let meta = test_cache(&config, &root).join(META_FILE_NAME);
        assert_eq!(load_meta(&meta).mtimes.len(), 1);
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);

        std::fs::remove_file(&page).unwrap();
        symlink(&secret, &page).unwrap();
        // Even before the next update, the stale result cannot open a link.
        assert!(find(&config, &root, "ordinary", SearchScope::All).is_empty());
        update_index_in(&config, &root).unwrap();
        assert!(find(&config, &root, "ordinary", SearchScope::All).is_empty());
        assert!(find(&config, &root, "confidential", SearchScope::All).is_empty());
        assert!(load_meta(&meta).mtimes.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn refuses_linked_cache_roots_metadata_and_segments_without_touching_targets() {
        use std::os::unix::fs::symlink;
        for component in
            [INDEX_DIR_NAME, "", TANTIVY_SUBDIR_NAME, META_FILE_NAME, "tantivy/meta.json", "tantivy/.mind-pkm-index"]
        {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path().join("docs");
            let config = tmp.path().join("config/mind-pkm");
            let outside = tmp.path().join("outside");
            write(&root, "a.md", "hello");
            let keep = write(&outside, "tantivy/keep.txt", "keep");
            let cache = test_cache(&config, &root);
            let target = match component {
                INDEX_DIR_NAME => config.join(INDEX_DIR_NAME),
                "" => cache,
                name => cache.join(name),
            };
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            let source = match component {
                INDEX_DIR_NAME | "" => outside.clone(),
                TANTIVY_SUBDIR_NAME => outside.join("tantivy"),
                _ => outside.join("missing.txt"),
            };
            symlink(&source, &target).unwrap();
            assert!(update_index_in(&config, &root).is_err(), "{}", target.display());
            assert!(
                search_in(&config, &root, "hello", SearchSort::Relevance, SearchScope::All, 10).is_err(),
                "{}",
                target.display()
            );
            assert_eq!(std::fs::read_to_string(&keep).unwrap(), "keep", "{}", target.display());
            assert!(!outside.join("missing.txt").exists());
            assert!(target.symlink_metadata().unwrap().file_type().is_symlink());
        }
    }

    #[test]
    fn refuses_to_remove_an_unowned_nonempty_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let (d, config) = (&tmp.path().join("docs"), &tmp.path().join("config/mind-pkm"));
        write(d, "a.md", "hello");
        let keep = write(&test_cache(config, d), "tantivy/keep.txt", "keep");
        assert!(update_index_in(config, d).unwrap_err().contains("not owned"));
        assert_eq!(std::fs::read_to_string(keep).unwrap(), "keep");
    }

    #[test]
    fn rebuilds_legacy_indexes_and_discards_previously_indexed_external_content() {
        let tmp = tempfile::tempdir().unwrap();
        let (d, config) = (&tmp.path().join("docs"), &tmp.path().join("config/mind-pkm"));
        let page = write(d, "a.md", "confidential");
        update_index_in(config, d).unwrap();
        let cache = test_cache(config, d);
        let owner = cache.join(TANTIVY_SUBDIR_NAME).join(OWNER_FILE_NAME);
        let meta = cache.join(META_FILE_NAME);
        // Simulate the old cache format, including a retained preview that a
        // previous version could have collected through a link.
        std::fs::remove_file(&owner).unwrap();
        std::fs::write(&meta, "version=1\n").unwrap();
        std::fs::write(page, "ordinary").unwrap();
        assert!(find(config, d, "confidential", SearchScope::All).is_empty());
        update_index_in(config, d).unwrap();
        assert!(find(config, d, "confidential", SearchScope::All).is_empty());
        assert_eq!(paths(&find(config, d, "ordinary", SearchScope::All)), vec!["a.md"]);
        assert!(owner.is_file());
        // An owned cache with corrupt metadata also recovers normally.
        std::fs::write(meta, "broken").unwrap();
        update_index_in(config, d).unwrap();
        assert_eq!(paths(&find(config, d, "ordinary", SearchScope::All)), vec!["a.md"]);
    }

    #[test]
    fn sorts_by_modified_time() {
        let tmp = tempfile::tempdir().unwrap();
        let (d, config) = (&tmp.path().join("docs"), &tmp.path().join("config/mind-pkm"));
        let old = write(d, "old.md", "shared");
        write(d, "new.md", "shared shared shared");
        let past = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        std::fs::File::options().write(true).open(&old).unwrap().set_modified(past).unwrap();
        update_index_in(config, d).unwrap();
        let hits = search_in(config, d, "shared", SearchSort::Modified, SearchScope::All, 10).unwrap();
        assert_eq!(hits.iter().map(|h| h.relative_path.as_str()).collect::<Vec<_>>(), vec!["new.md", "old.md"]);
    }

    #[test]
    fn single_line_keeps_highlights() {
        let s = Snippet { text: "a\n\n  bc".into(), highlights: vec![Range { start: 5, end: 7 }] };
        let one = single_line(&s);
        assert_eq!(one.text, "a bc");
        assert_eq!(&one.text[one.highlights[0].clone()], "bc");
    }
}
