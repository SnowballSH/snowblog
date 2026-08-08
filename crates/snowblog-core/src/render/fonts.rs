use std::path::PathBuf;

use typst_kit::fonts::FontStore;

pub fn load_fonts(extra_font_dirs: &[PathBuf]) -> FontStore {
    let mut store = FontStore::new();
    store.extend(typst_kit::fonts::embedded());
    for dir in extra_font_dirs {
        store.extend(typst_kit::fonts::scan(dir));
    }
    store
}
