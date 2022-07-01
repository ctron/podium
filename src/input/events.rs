use crate::input::key::Key;
use crate::input::InputEvent;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::time::{interval, Interval};

/// A small event handler that wrap crossterm input and tick event. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: tokio::sync::mpsc::Receiver<InputEvent>,
    // Need to be kept around to prevent disposing the sender side.
    _tx: tokio::sync::mpsc::Sender<InputEvent>,
    // To stop the loop
    stop_capture: Arc<AtomicBool>,
    // render interval
    interval: Interval,
}

impl Events {
    /// Constructs an new instance of `Events` with the default config.
    pub fn new(render_rate: Duration) -> Events {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let stop_capture = Arc::new(AtomicBool::new(false));

        let event_tx = tx.clone();
        let event_stop_capture = stop_capture.clone();
        tokio::spawn(async move {
            loop {
                // poll for tick rate duration, if no event, sent tick event.
                if crossterm::event::poll(render_rate).unwrap() {
                    if let crossterm::event::Event::Key(key) = crossterm::event::read().unwrap() {
                        let key = Key::from(key);
                        if let Err(err) = event_tx.send(InputEvent::Input(key)).await {
                            log::error!("Oops (event)!, {}", err);
                        }
                    }
                }
                if event_stop_capture.load(Ordering::Relaxed) {
                    break;
                }
            }
        });

        Events {
            rx,
            _tx: tx,
            stop_capture,
            interval: interval(render_rate),
        }
    }

    /// Attempts to read an event.
    pub async fn next(&mut self) -> InputEvent {
        select! {
            msg = self.rx.recv() => msg.unwrap_or(InputEvent::Quit),
            _ = self.interval.tick() => InputEvent::Render,
        }
    }
}

impl Drop for Events {
    fn drop(&mut self) {
        self.stop_capture.store(true, Ordering::Relaxed)
    }
}
