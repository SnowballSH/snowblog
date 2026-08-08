use std::collections::HashMap;
use std::path::{Path, PathBuf};

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Feature, Library, LibraryExt, World};
use typst_html::{HtmlDocument, HtmlOptions};

struct SpikeWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
    main_source: Source,
    files: HashMap<String, Bytes>,
    package_root: PathBuf,
}

impl SpikeWorld {
    fn new(source_text: String, files: HashMap<String, Bytes>, package_root: PathBuf) -> Self {
        let library = Library::builder()
            .with_features([Feature::Html].into_iter().collect())
            .build();
        let mut book = FontBook::new();
        let mut fonts = Vec::new();
        for (font, info) in typst_kit::fonts::embedded() {
            book.push(info);
            fonts.push(font);
        }
        let main = RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new("/main.typ").expect("static path"),
        )
        .intern();
        let main_source = Source::new(main, source_text);
        Self {
            library: LazyHash::new(library),
            book: LazyHash::new(book),
            fonts,
            main,
            main_source,
            files,
            package_root,
        }
    }

    fn package_path(&self, id: FileId) -> Option<PathBuf> {
        match id.root() {
            VirtualRoot::Package(spec) => {
                let dir = self
                    .package_root
                    .join(spec.namespace.as_str())
                    .join(spec.name.as_str())
                    .join(spec.version.to_string());
                id.vpath().realize(&dir).ok()
            }
            VirtualRoot::Project => None,
        }
    }
}

impl World for SpikeWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main {
            return Ok(self.main_source.clone());
        }
        if let Some(path) = self.package_path(id) {
            let text = std::fs::read_to_string(&path).map_err(|e| FileError::from_io(e, &path))?;
            return Ok(Source::new(id, text));
        }
        Err(FileError::AccessDenied)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.main {
            return Ok(Bytes::from_string(self.main_source.text().to_string()));
        }
        if let Some(path) = self.package_path(id) {
            let data = std::fs::read(&path).map_err(|e| FileError::from_io(e, &path))?;
            return Ok(Bytes::new(data));
        }
        if let VirtualRoot::Project = id.root() {
            let key = id
                .vpath()
                .get_with_slash()
                .trim_start_matches('/')
                .to_string();
            if let Some(bytes) = self.files.get(&key) {
                return Ok(bytes.clone());
            }
            return Err(FileError::NotFound(PathBuf::from(key)));
        }
        Err(FileError::AccessDenied)
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
        None
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let source_path = PathBuf::from(args.next().expect("usage: spike <file.typ> [assets-dir]"));
    let assets_dir = args.next().map(PathBuf::from);

    let source_text = std::fs::read_to_string(&source_path).expect("read source");
    let mut files = HashMap::new();
    if let Some(dir) = assets_dir {
        for entry in std::fs::read_dir(&dir).expect("read assets dir") {
            let entry = entry.expect("dir entry");
            if entry.file_type().expect("file type").is_file() {
                let name = entry.file_name().to_string_lossy().to_string();
                let data = std::fs::read(entry.path()).expect("read asset");
                files.insert(format!("assets/{name}"), Bytes::new(data));
            }
        }
    }

    let package_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/packages");
    let world = SpikeWorld::new(source_text, files, package_root);

    let result = typst::compile::<HtmlDocument>(&world);
    for warning in &result.warnings {
        eprintln!("warning: {}", warning.message);
    }
    match result.output {
        Ok(document) => {
            let html =
                typst_html::html(&document, &HtmlOptions { pretty: false }).expect("encode html");
            println!("{html}");
        }
        Err(errors) => {
            for error in &errors {
                eprintln!("error: {}", error.message);
            }
            std::process::exit(1);
        }
    }
}
