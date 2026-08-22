use std::path::{Path, PathBuf};

use gpui::{Context, PromptLevel, Window};

use crate::media::{
    copy_into, count_tree, duplicate, move_into, rename_with, restore_path, trash_path, under_root,
    Collision, Entry, FsError, MediaKind,
};

use super::Gallery;

#[derive(Clone)]
pub(super) enum Clip {
    Cut(Vec<PathBuf>),
    Copy(Vec<PathBuf>),
}

impl Clip {
    fn paths(&self) -> &[PathBuf] {
        match self {
            Self::Cut(p) | Self::Copy(p) => p,
        }
    }
}

#[derive(Clone)]
pub(super) enum UndoItem {
    PutBack { from: PathBuf, to: PathBuf },
    Trash { path: PathBuf },
}

impl UndoItem {
    fn current(&self) -> &Path {
        match self {
            Self::PutBack { from, .. } => from,
            Self::Trash { path } => path,
        }
    }
}

pub(super) struct Toast {
    pub(super) text: String,
    pub(super) undo: Option<Vec<UndoItem>>,
    pub(super) gen: u64,
}

#[derive(Clone)]
pub(super) enum PendingKind {
    Move { dest_dir: PathBuf },
    Copy { dest_dir: PathBuf },
    Rename { new_name: String },
}

pub(super) struct CollisionAsk {
    pub(super) kind: PendingKind,
    pub(super) from: PathBuf,
    pub(super) remaining: Vec<PathBuf>,
    done: Vec<UndoItem>,
    pub(super) apply_all: bool,
    folder: PathBuf,
    next: Option<PathBuf>,
    reopen: bool,
    restore_clip: Option<Clip>,
}

impl Gallery {
    pub(super) fn is_cut(&self, path: &Path) -> bool {
        matches!(&self.clip, Some(Clip::Cut(paths)) if paths.iter().any(|p| p == path))
    }

    pub(super) fn collision_name(&self) -> String {
        self.collision
            .as_ref()
            .and_then(|c| c.from.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("item")
            .to_string()
    }

    fn neighbor_path(&self, removing: &[PathBuf]) -> Option<PathBuf> {
        let vis = self.visible_indices();
        let keep: Vec<usize> = vis
            .into_iter()
            .filter(|&i| {
                self.entries
                    .get(i)
                    .is_some_and(|e| !removing.iter().any(|p| p == e.path()))
            })
            .collect();
        let start = self.selected.or(self.focused).unwrap_or(0);
        keep.iter()
            .copied()
            .find(|&i| i >= start)
            .or_else(|| keep.iter().copied().rev().find(|&i| i < start))
            .and_then(|i| self.entries.get(i).map(|e| e.path().to_path_buf()))
    }

    fn show_toast(
        &mut self,
        text: impl Into<String>,
        undo: Option<Vec<UndoItem>>,
        cx: &mut Context<Self>,
    ) {
        self.toast_gen += 1;
        let gen = self.toast_gen;
        self.toast = Some(Toast {
            text: text.into(),
            undo,
            gen,
        });
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(8))
                .await;
            this.update(cx, |this, cx| {
                if this.toast.as_ref().map(|t| t.gen) == Some(gen) {
                    this.toast = None;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn finish_job(
        &mut self,
        done: Vec<UndoItem>,
        folder: PathBuf,
        focus: Option<PathBuf>,
        reopen: bool,
        message: String,
        cx: &mut Context<Self>,
    ) {
        self.collision = None;
        if done.is_empty() {
            cx.notify();
            return;
        }
        let focus = focus
            .or_else(|| done.last().map(|item| item.current().to_path_buf()))
            .unwrap_or_else(|| folder.clone());
        self.show_toast(message, Some(done), cx);
        self.reload_listing(folder, focus, reopen, cx);
    }

    fn run_one(
        &self,
        kind: &PendingKind,
        from: &Path,
        collision: Collision,
    ) -> Result<UndoItem, FsError> {
        match kind {
            PendingKind::Move { dest_dir } => {
                let dest = move_into(from, dest_dir, &self.root, collision)?;
                Ok(UndoItem::PutBack {
                    from: dest,
                    to: from.to_path_buf(),
                })
            }
            PendingKind::Copy { dest_dir } => {
                let dest = copy_into(from, dest_dir, &self.root, collision)?;
                Ok(UndoItem::Trash { path: dest })
            }
            PendingKind::Rename { new_name } => {
                let dest = rename_with(from, new_name, &self.root, collision)?;
                Ok(UndoItem::PutBack {
                    from: dest,
                    to: from.to_path_buf(),
                })
            }
        }
    }

    fn pump_job(
        &mut self,
        kind: PendingKind,
        mut remaining: Vec<PathBuf>,
        mut done: Vec<UndoItem>,
        apply: Option<Collision>,
        folder: PathBuf,
        next: Option<PathBuf>,
        reopen: bool,
        restore_clip: Option<Clip>,
        cx: &mut Context<Self>,
    ) {
        while let Some(from) = remaining.first().cloned() {
            let collision = apply.unwrap_or(Collision::Fail);
            match self.run_one(&kind, &from, collision) {
                Ok(item) => {
                    remaining.remove(0);
                    done.push(item);
                }
                Err(FsError::Collision) => {
                    remaining.remove(0);
                    self.collision = Some(CollisionAsk {
                        kind,
                        from,
                        remaining,
                        done,
                        apply_all: false,
                        folder,
                        next,
                        reopen,
                        restore_clip,
                    });
                    cx.notify();
                    return;
                }
                Err(err) => {
                    remaining.remove(0);
                    if done.is_empty() && remaining.is_empty() {
                        self.show_toast(err.as_str(), None, cx);
                        return;
                    }
                }
            }
        }
        let n = done.len();
        let dest_here = match &kind {
            PendingKind::Move { dest_dir } | PendingKind::Copy { dest_dir } => dest_dir == &folder,
            PendingKind::Rename { .. } => true,
        };
        let focus = if dest_here { None } else { next };
        let message = match &kind {
            PendingKind::Move { .. } => format!("Moved {n} items"),
            PendingKind::Copy { .. } => format!("Copied {n} items"),
            PendingKind::Rename { .. } => "Renamed".into(),
        };
        self.finish_job(done, folder, focus, reopen, message, cx);
    }

    pub(super) fn toggle_collision_all(&mut self, cx: &mut Context<Self>) {
        if let Some(ask) = &mut self.collision {
            ask.apply_all = !ask.apply_all;
            cx.notify();
        }
    }

    pub(super) fn cancel_collision(&mut self, cx: &mut Context<Self>) {
        if let Some(ask) = self.collision.take() {
            if let Some(clip) = ask.restore_clip {
                let mut paths = vec![ask.from];
                paths.extend(ask.remaining);
                self.clip = Some(match clip {
                    Clip::Cut(_) => Clip::Cut(paths),
                    Clip::Copy(_) => Clip::Copy(paths),
                });
            }
            if !ask.done.is_empty() {
                let n = ask.done.len();
                let dest_here = match &ask.kind {
                    PendingKind::Move { dest_dir } | PendingKind::Copy { dest_dir } => {
                        dest_dir == &ask.folder
                    }
                    PendingKind::Rename { .. } => true,
                };
                let focus = if dest_here { None } else { ask.next };
                let message = match ask.kind {
                    PendingKind::Move { .. } => format!("Moved {n} items"),
                    PendingKind::Copy { .. } => format!("Copied {n} items"),
                    PendingKind::Rename { .. } => "Renamed".into(),
                };
                self.finish_job(ask.done, ask.folder, focus, ask.reopen, message, cx);
                return;
            }
        }
        cx.notify();
    }

    pub(super) fn resolve_collision(&mut self, choice: Collision, cx: &mut Context<Self>) {
        let Some(mut ask) = self.collision.take() else {
            return;
        };
        match self.run_one(&ask.kind, &ask.from, choice) {
            Ok(item) => ask.done.push(item),
            Err(err) => {
                self.show_toast(err.as_str(), None, cx);
                return;
            }
        }
        let apply = if ask.apply_all { Some(choice) } else { None };
        self.pump_job(
            ask.kind,
            ask.remaining,
            ask.done,
            apply,
            ask.folder,
            ask.next,
            ask.reopen,
            ask.restore_clip,
            cx,
        );
    }

    fn ask_rename_collision(
        &mut self,
        from: PathBuf,
        new_name: String,
        reopen: bool,
        cx: &mut Context<Self>,
    ) {
        self.collision = Some(CollisionAsk {
            kind: PendingKind::Rename { new_name },
            from,
            remaining: Vec::new(),
            done: Vec::new(),
            apply_all: false,
            folder: self.folder.clone(),
            next: None,
            reopen,
            restore_clip: None,
        });
        cx.notify();
    }

    pub(super) fn begin_rename_with_collision(
        &mut self,
        from: PathBuf,
        new_name: String,
        reopen: bool,
        cx: &mut Context<Self>,
    ) {
        match rename_with(&from, &new_name, &self.root, Collision::Fail) {
            Ok(dest) => {
                self.show_toast(
                    "Renamed",
                    Some(vec![UndoItem::PutBack {
                        from: dest.clone(),
                        to: from,
                    }]),
                    cx,
                );
                self.reload_listing(self.folder.clone(), dest, reopen, cx);
            }
            Err(FsError::Collision) => self.ask_rename_collision(from, new_name, reopen, cx),
            Err(err) => self.show_toast(err.as_str(), None, cx),
        }
    }

    pub(super) fn move_to_trash(
        &mut self,
        _: &super::MoveToTrash,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.context = None;
        if self.name_kind.is_some() || self.collision.is_some() {
            return;
        }
        let paths = self.action_paths();
        let paths: Vec<PathBuf> = paths
            .into_iter()
            .filter(|p| p != &self.root && under_root(p, &self.root))
            .collect();
        if paths.is_empty() {
            return;
        }
        let extra: usize = paths
            .iter()
            .filter(|p| p.is_dir())
            .map(|p| count_tree(p))
            .sum();
        let need_confirm = paths.len() > 1 || extra > 0;
        if need_confirm {
            let n = paths.len();
            let detail = if paths.len() == 1 && extra > 0 {
                format!("Move this folder and {extra} items inside to Trash?")
            } else if extra > 0 {
                format!("Move {n} items ({extra} inside folders) to Trash?")
            } else {
                format!("Move {n} items to Trash?")
            };
            let rx = window.prompt(
                PromptLevel::Warning,
                "Move to Trash",
                Some(&detail),
                &["Move to Trash", "Cancel"],
                cx,
            );
            cx.spawn(async move |this, cx| {
                if rx.await.ok() != Some(0) {
                    return;
                }
                this.update(cx, |this, cx| this.run_trash(paths, cx)).ok();
            })
            .detach();
            return;
        }
        self.run_trash(paths, cx);
    }

    fn run_trash(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let next = self.neighbor_path(&paths);
        let mut folder = self.folder.clone();
        let mut reopen = self.selected.is_some()
            && next.as_ref().is_some_and(|p| {
                matches!(
                    self.entries.iter().find(|e| e.path() == p),
                    Some(Entry::Media(m)) if m.kind == MediaKind::Image
                )
            });
        let mut done = Vec::new();
        for path in &paths {
            if folder == *path || folder.starts_with(path) {
                folder = path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .filter(|p| under_root(p, &self.root))
                    .unwrap_or_else(|| self.root.clone());
                reopen = false;
            }
            match trash_path(path, &self.root) {
                Ok(loc) => done.push(UndoItem::PutBack {
                    from: loc,
                    to: path.clone(),
                }),
                Err(err) => {
                    if done.is_empty() {
                        self.show_toast(err.as_str(), None, cx);
                        return;
                    }
                }
            }
        }
        let n = done.len();
        self.finish_job(
            done,
            folder,
            next,
            reopen,
            format!("Moved {n} items to Trash"),
            cx,
        );
    }

    pub(super) fn duplicate_selected(
        &mut self,
        _: &super::Duplicate,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.context = None;
        if self.name_kind.is_some() || self.collision.is_some() || self.search_open {
            return;
        }
        let paths: Vec<PathBuf> = self
            .action_paths()
            .into_iter()
            .filter(|p| p.is_file())
            .collect();
        if paths.is_empty() {
            self.show_toast("Can't duplicate folders.", None, cx);
            return;
        }
        let mut done = Vec::new();
        for path in &paths {
            match duplicate(path, &self.root) {
                Ok(dest) => done.push(UndoItem::Trash { path: dest }),
                Err(err) => {
                    if done.is_empty() {
                        self.show_toast(err.as_str(), None, cx);
                        return;
                    }
                }
            }
        }
        let n = done.len();
        let last = done.last().map(|i| i.current().to_path_buf());
        self.finish_job(
            done,
            self.folder.clone(),
            last,
            false,
            format!("Duplicated {n} items"),
            cx,
        );
    }

    pub(super) fn cut_selected(
        &mut self,
        _: &super::CutSelection,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.context = None;
        if self.name_kind.is_some() || self.collision.is_some() {
            return;
        }
        let paths = self.action_paths();
        if paths.is_empty() {
            return;
        }
        self.clip = Some(Clip::Cut(paths));
        cx.notify();
    }

    pub(super) fn copy_selected(
        &mut self,
        _: &super::CopySelection,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.context = None;
        if self.name_kind.is_some() || self.collision.is_some() {
            return;
        }
        let paths = self.action_paths();
        if paths.is_empty() {
            return;
        }
        self.clip = Some(Clip::Copy(paths));
        cx.notify();
    }

    pub(super) fn paste_clipboard(
        &mut self,
        _: &super::PasteSelection,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.context = None;
        if self.name_kind.is_some() || self.collision.is_some() {
            return;
        }
        if self.prefs.flat_mode {
            self.show_toast("Paste into a folder in Folders mode.", None, cx);
            return;
        }
        let Some(clip) = self.clip.clone() else {
            return;
        };
        let paths = clip.paths().to_vec();
        let restore_clip = Some(clip.clone());
        let kind = match clip {
            Clip::Cut(_) => {
                self.clip = None;
                PendingKind::Move {
                    dest_dir: self.folder.clone(),
                }
            }
            Clip::Copy(_) => PendingKind::Copy {
                dest_dir: self.folder.clone(),
            },
        };
        self.pump_job(
            kind,
            paths,
            Vec::new(),
            None,
            self.folder.clone(),
            None,
            false,
            restore_clip,
            cx,
        );
    }

    fn pick_dest(&mut self, copy: bool, cx: &mut Context<Self>) {
        self.context = None;
        if self.name_kind.is_some() || self.collision.is_some() {
            return;
        }
        let paths = self.action_paths();
        if paths.is_empty() {
            return;
        }
        let rx = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(if copy {
                "Copy to".into()
            } else {
                "Move to".into()
            }),
        });
        cx.spawn(async move |this, cx| {
            let Ok(Ok(Some(picked))) = rx.await else {
                return;
            };
            let Some(dest) = picked.into_iter().next() else {
                return;
            };
            this.update(cx, |this, cx| {
                if !under_root(&dest, &this.root) {
                    this.show_toast("That stays inside this library.", None, cx);
                    return;
                }
                let next = this.neighbor_path(&paths);
                let kind = if copy {
                    PendingKind::Copy { dest_dir: dest }
                } else {
                    PendingKind::Move { dest_dir: dest }
                };
                this.pump_job(
                    kind,
                    paths,
                    Vec::new(),
                    None,
                    this.folder.clone(),
                    next,
                    false,
                    None,
                    cx,
                );
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn move_to(&mut self, _: &super::MoveTo, _: &mut Window, cx: &mut Context<Self>) {
        self.pick_dest(false, cx);
    }

    pub(super) fn copy_to(&mut self, _: &super::CopyTo, _: &mut Window, cx: &mut Context<Self>) {
        self.pick_dest(true, cx);
    }

    pub(super) fn undo_last(&mut self, _: &super::Undo, _: &mut Window, cx: &mut Context<Self>) {
        let Some(toast) = self.toast.take() else {
            return;
        };
        let Some(items) = toast.undo else {
            return;
        };
        let mut last = None;
        for item in items.into_iter().rev() {
            let result = match item {
                UndoItem::PutBack { from, to } => restore_path(&from, &to, &self.root),
                UndoItem::Trash { path } => trash_path(&path, &self.root),
            };
            match result {
                Ok(path) => last = Some(path),
                Err(_) => {
                    self.show_toast("Undo isn't possible.", None, cx);
                    self.reload_listing(
                        self.folder.clone(),
                        last.unwrap_or_else(|| self.folder.clone()),
                        false,
                        cx,
                    );
                    return;
                }
            }
        }
        let focus = last.unwrap_or_else(|| self.folder.clone());
        self.reload_listing(self.folder.clone(), focus, false, cx);
    }
}
