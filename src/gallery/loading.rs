use gpui::{App, AppContext, Context, ImageSource};

use crate::media::{load_or_make_thumb, Entry};

use super::Gallery;

mod queue;
pub(super) use queue::ThumbRequests;

impl Gallery {
    pub(super) fn queue_thumbs(
        &mut self,
        indices: impl Iterator<Item = usize>,
        cx: &mut Context<Self>,
    ) {
        let demand: Vec<_> = indices
            .filter_map(|index| match self.entries.get(index) {
                Some(Entry::Media(item)) => Some(item.path.clone()),
                _ => None,
            })
            .collect();
        if self.thumb_demand != demand {
            self.thumb_demand = demand;
            let obsolete: Vec<_> = self
                .thumbs
                .keys()
                .filter(|path| !self.thumb_demand.contains(path))
                .cloned()
                .collect();
            for path in obsolete {
                if let Some(thumb) = self.thumbs.remove(&path) {
                    ImageSource::from(thumb).remove_asset(cx);
                }
            }
            cx.notify();
        }
        let paths = self
            .thumb_demand
            .iter()
            .filter(|path| !self.thumbs.contains_key(*path) && !self.failed_thumbs.contains(*path))
            .cloned()
            .collect();
        self.thumb_requests.set_demand(self.load_gen, paths);
        self.start_thumb_jobs(cx);
    }

    fn start_thumb_jobs(&mut self, cx: &mut Context<Self>) {
        while let Some(request) = self.thumb_requests.next_request() {
            cx.spawn(async move |this, cx| {
                let id = request.id;
                let thumb = cx
                    .background_spawn(async move {
                        // A queued background task can become obsolete before it starts.
                        request
                            .is_wanted()
                            .then(|| load_or_make_thumb(&request.path))
                            .flatten()
                    })
                    .await;
                this.update(cx, |this, cx| {
                    if let Some(path) = this.thumb_requests.complete(id) {
                        if let Some(thumb) = thumb {
                            this.thumbs.insert(path, thumb);
                        } else {
                            this.failed_thumbs.insert(path);
                        }
                        cx.notify();
                    }
                    this.start_thumb_jobs(cx);
                })
                .ok();
            })
            .detach();
        }
    }

    pub(super) fn clear_thumbs(&mut self, cx: &mut App) {
        for (_, thumb) in self.thumbs.drain() {
            ImageSource::from(thumb).remove_asset(cx);
        }
        self.failed_thumbs.clear();
    }
}
