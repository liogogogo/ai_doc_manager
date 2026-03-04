use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WatchError {
    #[error("Watch error: {0}")]
    Notify(#[from] notify::Error),
}

pub struct FileWatcherService {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<notify::Result<Event>>,
}

impl FileWatcherService {
    pub fn new(path: &Path) -> Result<Self, WatchError> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )?;

        watcher.watch(path, RecursiveMode::Recursive)?;

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    pub fn try_recv(&self) -> Option<Event> {
        self.rx.try_recv().ok().and_then(|r| r.ok())
    }
}
