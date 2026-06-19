use std::path::{Path, PathBuf};

use crate::paths::{host_root, portable_source_dir};

const RETRO_GO_FORK_DIR_NAME: &str = "game-and-watch-retro-go-sylverb";

fn local_retro_go_candidate_from(start: PathBuf) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        let candidate = ancestor.join(RETRO_GO_FORK_DIR_NAME);
        if candidate.join("Makefile").is_file() {
            return Some(candidate);
        }
    }
    None
}

pub(crate) fn locate_retro_go_repo() -> Option<PathBuf> {
    [
        host_root().join(RETRO_GO_FORK_DIR_NAME),
        portable_source_dir().join(RETRO_GO_FORK_DIR_NAME),
    ]
    .into_iter()
    .find(|path| path.join("Makefile").is_file())
    .or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .and_then(local_retro_go_candidate_from)
    })
    .or_else(|| std::env::current_dir().ok().and_then(local_retro_go_candidate_from))
    .or_else(|| local_retro_go_candidate_from(PathBuf::from(env!("CARGO_MANIFEST_DIR"))))
}
