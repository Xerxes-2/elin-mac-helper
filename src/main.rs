use anyhow::{Context, Result};
use log::{error, info, warn};
use notify::{Event, RecursiveMode, Watcher};
use std::{sync::mpsc, thread};

fn handle_event(event: notify::Event, path: &std::path::Path) -> Result<()> {
    if event.need_rescan() {
        warn!("Some events may be lost.");
    }
    let event_path = event.paths.get(0).context("No path")?.strip_prefix(path)?;
    let event_path_parent = event_path.parent().context("No parent")?;
    if event.kind.is_remove() {
        warn!("Removed: {:?}", event_path);
        if event_path_parent.read_dir()?.next().is_none() {
            warn!("Removing parent: {:?}", event_path_parent);
            std::fs::remove_dir(event_path_parent)?;
        }
    } else {
        info!("Event {:?}: {:?}", event.kind, event_path);
    }
    Ok(())
}

fn main() -> Result<()> {
    env_logger::builder()
        .filter(Some("elin_mac_helper"), log::LevelFilter::Info)
        .format_timestamp(None)
        .format_target(false)
        .init();
    info!("elin-mac-helper started");
    let args: Vec<String> = std::env::args().collect();
    let target = args.get(1).map(|s| s.as_str()).unwrap_or(".");
    let full_path = std::path::Path::new(target);
    // change to target directory
    std::env::set_current_dir(full_path)?;
    let path = std::path::Path::new(".");
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path.read_dir()?.next().is_none() {
                std::fs::remove_dir(&path)?;
                info!("Removed: {:?}", path.strip_prefix(&path).unwrap());
            }
        }
    }
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(path, RecursiveMode::Recursive)?;
    info!("Watching: {:?}", full_path);
    thread::scope(|s| {
        for res in rx {
            match res {
                Ok(event) => {
                    s.spawn(move || {
                        handle_event(event, full_path).unwrap_or_else(|e| error!("{:?}", e));
                    });
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
        }
    });
    Ok(())
}
