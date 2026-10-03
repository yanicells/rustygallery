use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use gpui::{prelude::*, Context};

use crate::media::listing_stamp;

use super::Gallery;

struct WatchSnapshot {
    generation: u64,
    folder: PathBuf,
    flat: bool,
}

impl WatchSnapshot {
    fn is_current(&self, generation: u64, folder: &Path, flat: bool) -> bool {
        self.generation == generation && self.folder == folder && self.flat == flat
    }
}

impl Gallery {
    pub(super) fn start_watch(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(Duration::from_secs(3)).await;
            let Some((snapshot, ignore, skip)) = this
                .update(cx, |this, _| {
                    Some((
                        WatchSnapshot {
                            generation: this.load_gen,
                            folder: this.folder.clone(),
                            flat: this.prefs.flat_mode,
                        },
                        this.prefs.ignore.clone(),
                        this.loading
                            || this.name_kind.is_some()
                            || this.collision.is_some()
                            || this.search_open,
                    ))
                })
                .ok()
                .flatten()
            else {
                break;
            };
            if skip {
                continue;
            }
            let folder = snapshot.folder.clone();
            let flat = snapshot.flat;
            let next = cx
                .background_spawn(async move { listing_stamp(&folder, flat, &ignore) })
                .await;
            let cont = this
                .update(cx, |this, cx| {
                    if !snapshot.is_current(this.load_gen, &this.folder, this.prefs.flat_mode)
                        || this.loading
                        || this.name_kind.is_some()
                        || this.collision.is_some()
                        || this.search_open
                    {
                        return true;
                    }
                    if this.watch_stamp == Some(next) {
                        return true;
                    }
                    if this.watch_stamp.is_none() {
                        this.watch_stamp = Some(next);
                        return true;
                    }
                    this.watch_stamp = Some(next);
                    let focus = this
                        .selected
                        .or(this.focused)
                        .and_then(|i| this.entries.get(i))
                        .map(|e| e.path().to_path_buf())
                        .unwrap_or_else(|| this.folder.clone());
                    let open = this.selected.is_some();
                    this.reload_listing(this.folder.clone(), focus, open, cx);
                    true
                })
                .unwrap_or(false);
            if !cont {
                break;
            }
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::WatchSnapshot;
    use std::path::{Path, PathBuf};

    #[test]
    fn completed_watch_scan_must_match_folder_mode_and_load_generation() {
        let snapshot = WatchSnapshot {
            generation: 7,
            folder: PathBuf::from("/library/a"),
            flat: false,
        };
        assert!(snapshot.is_current(7, Path::new("/library/a"), false));
        assert!(!snapshot.is_current(8, Path::new("/library/a"), false));
        assert!(!snapshot.is_current(7, Path::new("/library/b"), false));
        assert!(!snapshot.is_current(7, Path::new("/library/a"), true));
    }
}
