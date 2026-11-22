//! PDF rendering with an embedded Typst compiler.
//!
//! A template is a Typst file. It can read three virtual JSON files:
//! `/view.json` (the prepared [`View`]), `/profile.json` (the raw profile) and
//! `/options.json` (language and page limit). Templates cannot access the disk.

use std::collections::HashMap;
use std::sync::OnceLock;

use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World, WorldExt};
use typst_kit::fonts::FontStore;
use typst_layout::PagedDocument;

use super::view::View;
use crate::template::Template;
use crate::{Error, Profile};

pub fn render(
    template: &Template,
    profile: &Profile,
    view: &View,
    options: &serde_json::Value,
) -> Result<Vec<u8>, Error> {
    let world = TemplateWorld::new(
        template,
        [
            ("view.json", serde_json::to_vec(view)?),
            ("profile.json", serde_json::to_vec(profile)?),
            ("options.json", serde_json::to_vec(options)?),
        ],
    )?;
    let document: PagedDocument = typst::compile(&world)
        .output
        .map_err(|diags| world.error(&diags))?;
    typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|diags| world.error(&diags))
}

/// Fonts are scanned once per process: the bundled ones first, so templates render
/// the same everywhere, then system fonts.
fn fonts() -> &'static FontStore {
    static FONTS: OnceLock<FontStore> = OnceLock::new();
    FONTS.get_or_init(|| {
        let mut store = FontStore::new();
        store.extend(typst_kit::fonts::embedded());
        store.extend(typst_kit::fonts::system());
        store
    })
}

fn library() -> &'static LazyHash<Library> {
    static LIBRARY: OnceLock<LazyHash<Library>> = OnceLock::new();
    LIBRARY.get_or_init(|| LazyHash::new(Library::default()))
}

struct TemplateWorld {
    main: Source,
    files: HashMap<FileId, Bytes>,
}

impl TemplateWorld {
    fn new<const N: usize>(
        template: &Template,
        files: [(&str, Vec<u8>); N],
    ) -> Result<Self, Error> {
        let main = Source::new(file_id("main.typ")?, template.source().to_owned());
        let files = files
            .into_iter()
            .map(|(name, data)| Ok((file_id(name)?, Bytes::new(data))))
            .collect::<Result<_, Error>>()?;
        Ok(Self { main, files })
    }

    fn error(&self, diags: &[SourceDiagnostic]) -> Error {
        let messages: Vec<String> = diags
            .iter()
            .map(|d| {
                let line = d
                    .span
                    .id()
                    .filter(|id| *id == self.main.id())
                    .and_then(|_| self.range(d.span))
                    .and_then(|r| self.main.lines().byte_to_line(r.start))
                    .map(|l| format!("line {}: ", l + 1))
                    .unwrap_or_default();
                format!("{line}{}", d.message)
            })
            .collect();
        Error::Render(format!("template: {}", messages.join("; ")))
    }
}

fn file_id(name: &str) -> Result<FileId, Error> {
    let vpath = VirtualPath::new(name).map_err(|e| Error::Render(format!("{name}: {e:?}")))?;
    Ok(RootedPath::new(VirtualRoot::Project, vpath).intern())
}

impl World for TemplateWorld {
    fn library(&self) -> &LazyHash<Library> {
        library()
    }

    fn book(&self) -> &LazyHash<FontBook> {
        fonts().book()
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            Ok(self.main.clone())
        } else {
            Err(not_found(id))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.get(&id).cloned().ok_or_else(|| not_found(id))
    }

    fn font(&self, index: usize) -> Option<Font> {
        fonts().font(index)
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

fn not_found(id: FileId) -> FileError {
    FileError::NotFound(id.vpath().get_without_slash().into())
}
