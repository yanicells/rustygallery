mod app;
mod gallery;
mod media;
mod prefs;
mod ui;

use gpui::Application;

fn main() {
    let folder = app::resolve_folder();
    Application::new().run(move |cx| app::start(folder, cx));
}
