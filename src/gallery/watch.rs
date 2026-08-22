use std::time::Duration;

use gpui::{prelude::*, Context};

use crate::media::listing_stamp;

use super::Gallery;

impl Gallery {
    pub(super) fn start_watch(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(800))
                .await;
            let Some((folder, flat, ignore, skip)) = this
                .update(cx, |this, _| {
                    Some((
                        this.folder.clone(),
                        this.prefs.flat_mode,
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
            let next = cx
                .background_spawn(async move { listing_stamp(&folder, flat, &ignore) })
                .await;
            let cont = this
                .update(cx, |this, cx| {
                    if this.loading || this.name_kind.is_some() || this.collision.is_some() {
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
