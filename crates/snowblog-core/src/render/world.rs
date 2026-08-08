use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use typst::Library;
use typst::World;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst_kit::fonts::FontStore;

pub struct PostWorld {
    library: Arc<LazyHash<Library>>,
    fonts: Arc<FontStore>,
    main: FileId,
    main_source: Source,
    assets: HashMap<String, Bytes>,
    package_root: PathBuf,
}

impl PostWorld {
    pub fn new(
        library: Arc<LazyHash<Library>>,
        fonts: Arc<FontStore>,
        source_text: String,
        assets: HashMap<String, Bytes>,
        package_root: PathBuf,
    ) -> Self {
        let main = RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new("/main.typ").expect("static path"),
        )
        .intern();
        let main_source = Source::new(main, source_text);
        Self {
            library,
            fonts,
            main,
            main_source,
            assets,
            package_root,
        }
    }

    fn package_path(&self, id: FileId) -> FileResult<Option<PathBuf>> {
        match id.root() {
            VirtualRoot::Package(spec) => {
                let dir = self
                    .package_root
                    .join(spec.namespace.as_str())
                    .join(spec.name.as_str())
                    .join(spec.version.to_string());
                if !dir.is_dir() {
                    return Err(FileError::Other(Some(
                        format!(
                            "package @{}/{}:{} is not vendored",
                            spec.namespace, spec.name, spec.version
                        )
                        .into(),
                    )));
                }
                let path = id
                    .vpath()
                    .realize(&dir)
                    .map_err(|_| FileError::AccessDenied)?;
                Ok(Some(path))
            }
            VirtualRoot::Project => Ok(None),
        }
    }
}

impl World for PostWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main {
            return Ok(self.main_source.clone());
        }
        if let Some(path) = self.package_path(id)? {
            let text = std::fs::read_to_string(&path).map_err(|e| FileError::from_io(e, &path))?;
            return Ok(Source::new(id, text));
        }
        Err(FileError::AccessDenied)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.main {
            return Ok(Bytes::from_string(self.main_source.text().to_string()));
        }
        if let Some(path) = self.package_path(id)? {
            let data = std::fs::read(&path).map_err(|e| FileError::from_io(e, &path))?;
            return Ok(Bytes::new(data));
        }
        let key = id.vpath().get_with_slash().trim_start_matches('/');
        match self.assets.get(key) {
            Some(bytes) => Ok(bytes.clone()),
            None => Err(FileError::NotFound(PathBuf::from(key))),
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}
