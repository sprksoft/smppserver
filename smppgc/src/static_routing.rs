use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use lmetrics::metrics;
use log::*;
use rocket::routes;
use rocket::{fairing::AdHoc, fs::NamedFile, State};
use rocket::{get, serde::Deserialize};

metrics! {
    pub counter static_req_total("Total amount of requests to static resources", []);
}

#[derive(Default, Debug)]
pub struct FileIndex {
    index: HashSet<PathBuf>,
    pub root_dir: PathBuf,
}
impl FileIndex {
    pub fn build_from_dir(dir: PathBuf) -> Self {
        let mut index = HashSet::new();
        Self::index_content(&dir, &mut index).unwrap_or_else(|err| {
            error!("Failed to index directory '{}'\n{}", dir.display(), err);
        });
        Self {
            index,
            root_dir: dir,
        }
    }

    fn index_content(dir: &Path, index: &mut HashSet<PathBuf>) -> std::io::Result<()> {
        for child in std::fs::read_dir(dir)? {
            let child = child?;
            let meta = child.metadata()?;
            if meta.is_symlink() {
                continue;
            };
            if meta.is_dir() {
                Self::index_content(&child.path(), index)?;
            } else {
                index.insert(child.path());
            }
        }

        Ok(())
    }
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct StaticConfig {
    pub static_dir: PathBuf,
}

#[get("/<path..>", rank = 0)]
async fn static_get(path: PathBuf, file_index: &State<FileIndex>) -> Option<NamedFile> {
    static_req_total::inc();
    let path = Path::new(&file_index.root_dir).join(&path);
    if !file_index.index.contains(&path) {
        return None;
    }
    NamedFile::open(path).await.ok()
}

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("static routing", |r| async {
        let config = r
            .figment()
            .extract::<StaticConfig>()
            .expect("static_dir value is required for static routing");
        let mut path = config.static_dir;
        if path.is_relative() {
            path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        }
        r.manage(FileIndex::build_from_dir(path))
            .mount("/static", routes![static_get])
    })
}
