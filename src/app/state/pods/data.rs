use crate::k8s::ago;
use chrono::Utc;
use k8s_openapi::{
    api::core::v1::{ContainerState, ContainerStatus, PodStatus},
    apimachinery::pkg::apis::meta::v1::Time,
};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug)]
pub enum PodState {
    Pending,
    ContainerCreating,
    PodInitializing,
    Running,
    Error,
    CrashLoopBackOff,
    Terminating,
    Unknown,
    Other(String),
}

impl From<&str> for PodState {
    fn from(reason: &str) -> Self {
        match reason {
            "Pending" => Self::Pending,
            "ContainerCreating" => Self::ContainerCreating,
            "PodInitializing" => Self::PodInitializing,
            "Running" => Self::Running,
            "Error" => Self::Error,
            "CrashLoopBackOff" => Self::CrashLoopBackOff,
            reason => Self::Other(reason.to_string()),
        }
    }
}

impl From<&String> for PodState {
    fn from(reason: &String) -> Self {
        reason.as_str().into()
    }
}

impl From<Option<&str>> for PodState {
    fn from(reason: Option<&str>) -> Self {
        match reason {
            Some(reason) => reason.into(),
            None => PodState::Unknown,
        }
    }
}

impl Default for PodState {
    fn default() -> Self {
        Self::Unknown
    }
}

impl Display for PodState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => Ok(()),
            Self::ContainerCreating => f.write_str("ContainerCreating"),
            Self::PodInitializing => f.write_str("PodInitializing"),
            Self::Running => f.write_str("Running"),
            Self::Pending => f.write_str("Pending"),
            Self::Error => f.write_str("Error"),
            Self::CrashLoopBackOff => f.write_str("CrashLoopBackOff"),
            Self::Terminating => f.write_str("Terminating"),
            Self::Other(state) => f.write_str(state),
        }
    }
}

pub fn make_state(status: &PodStatus) -> PodState {
    // get all non-ready containers

    let mut containers: Vec<_> =
        with_last_changed(all_containers(status).filter(|c| !c.ready)).collect();

    // sort by latest change

    containers.sort_unstable_by(|a, b| a.0 .0.cmp(&b.0 .0));
    if let Some(container) = containers.first().map(|c| c.1) {
        let reason = match (
            container.state.as_ref().and_then(|s| s.waiting.as_ref()),
            container.state.as_ref().and_then(|s| s.terminated.as_ref()),
        ) {
            (Some(waiting), _) => waiting.reason.as_ref(),
            (_, Some(terminated)) => terminated.reason.as_ref(),
            _ => None,
        };
        if let Some(reason) = reason {
            return reason.into();
        }
    }

    // eval status

    status.phase.as_deref().into()
}

pub fn make_ready(status: &PodStatus) -> Option<String> {
    if let Some(init_container_statuses) = &status.init_container_statuses {
        let total = init_container_statuses.len();
        let ready = init_container_statuses.iter().filter(|s| s.ready).count();
        if total != ready {
            return Some(format!("Init:{ready}/{total}"));
        }
    }
    if let Some(container_statuses) = &status.container_statuses {
        let total = container_statuses.len();
        let ready = container_statuses.iter().filter(|s| s.ready).count();
        Some(format!("{ready}/{total}"))
    } else {
        None
    }
}

pub fn make_restarts(status: &PodStatus) -> Option<String> {
    let mut containers: Vec<_> =
        with_last_changed(all_containers(status).filter(|c| c.restart_count > 0)).collect();

    // sort by latest change

    containers.sort_unstable_by(|a, b| a.0 .0.cmp(&b.0 .0));

    if let Some(c) = containers.first() {
        let cnt = c.1.restart_count;
        let when =
            c.1.last_state
                .as_ref()
                .and_then(|s| s.terminated.as_ref())
                .and_then(|t| t.finished_at.as_ref())
                .and_then(ago);
        match when {
            Some(when) => Some(format!("{cnt} ({when} ago)")),
            None => Some(cnt.to_string()),
        }
    } else {
        None
    }
}

pub fn all_containers(status: &PodStatus) -> impl Iterator<Item = &ContainerStatus> {
    status
        .init_container_statuses
        .iter()
        .flatten()
        .chain(status.container_statuses.iter().flatten())
}

pub fn with_last_changed<'c>(
    containers: impl Iterator<Item = &'c ContainerStatus>,
) -> impl Iterator<Item = (Time, &'c ContainerStatus)> {
    containers.filter_map(|c| c.last_change().map(|time| (time, c)))
}

/// get the last time something changed
pub trait LastChange {
    fn last_change(&self) -> Option<Time>;
}

impl LastChange for ContainerState {
    fn last_change(&self) -> Option<Time> {
        match (
            self.running.as_ref().and_then(|s| s.started_at.clone()),
            self.waiting.as_ref(),
            self.terminated.as_ref().and_then(|s| s.finished_at.clone()),
        ) {
            (Some(started), _, _) => Some(started),
            // waiting doesn't have a time stamp, the again, it is waiting right now
            (_, Some(_waiting), _) => Some(Time(Utc::now())),
            (_, _, Some(finished_at)) => Some(finished_at),
            _ => None,
        }
    }
}

impl LastChange for Option<ContainerState> {
    fn last_change(&self) -> Option<Time> {
        self.as_ref().and_then(|s| s.last_change())
    }
}

impl LastChange for ContainerStatus {
    fn last_change(&self) -> Option<Time> {
        match self.state.last_change() {
            Some(time) => Some(time),
            None => self.last_state.last_change(),
        }
    }
}
