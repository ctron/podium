mod deployments;
pub mod list;
mod pods;

pub use pods::*;

use crate::app::state::deployments::Deployments;
use crate::app::state::list::ListWatcher;
use crate::input::key::Key;
use crate::ui::StateRenderer;
use k8s_openapi::api::core::v1::Pod;

pub enum AppState {
    Initializing,
    Pods(ListWatcher<Pod>),
    Deployments(ListWatcher<Deployments>),
}

impl AppState {
    pub fn render<R: StateRenderer>(&self, r: R) {
        match self {
            Self::Pods(pods) => pods.render(r),
            Self::Deployments(deployments) => deployments.render(r),
            _ => {}
        }
    }

    pub async fn on_key(&self, key: Key) {
        match self {
            Self::Pods(pods) => {
                pods.on_key(key).await;
            }
            Self::Deployments(deployments) => {
                deployments.on_key(key).await;
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
