mod pods;

pub use pods::*;

use crate::input::key::Key;
use crate::ui::StateRenderer;

pub enum AppState {
    Initializing,
    Pods(Pods),
    Deployments,
}

impl AppState {
    pub fn render<R: StateRenderer>(&self, r: R) {
        match self {
            Self::Pods(pods) => pods.render(r),
            _ => {}
        }
    }

    pub async fn on_key(&self, key: Key) {
        match self {
            Self::Pods(pods) => {
                pods.on_key(key).await;
            }
            _ => {}
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::Initializing
    }
}
