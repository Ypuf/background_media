slint::include_modules!();

fn is_gif(p: &std::path::Path) -> bool {
  p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gif"))
}

fn main() {
  let ui = AppWindow::new().unwrap();
  let api = ui.global::<Api>();
  api.on_can_drop(|data| {
    data.file_paths().is_ok_and(|mut paths| paths.any(is_gif))
  });

  api.on_dropped(|data| {
    if let Ok(paths) = data.file_paths() {
      for p in paths.filter(|p| is_gif(p)) {
        println!("dropped: {}", p.display());
      }
    }
  });

  ui.run().unwrap();
}
