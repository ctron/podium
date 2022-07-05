mod data;

use data::*;

use crate::k8s::ago;
use crate::{app::state::list::ListResource, client::Client, input::key::Key};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{DeleteParams, Preconditions},
    Api, Resource, ResourceExt,
};
use std::{fmt::Debug, future::Future, hash::Hash, pin::Pin, sync::Arc};
use tui::{layout::*, style::*, widgets::*};

impl ListResource for Pod {
    type Resource = Self;
    type Message = Msg;

    fn render_table<'r, 'a>(items: &'r mut [Arc<Self::Resource>]) -> Table<'a>
    where
        <<Self as ListResource>::Resource as Resource>::DynamicType: Hash + Eq,
    {
        items.sort_unstable_by(|a, b| a.name().cmp(&b.name()));

        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let normal_style = Style::default();
        let header_cells = ["Name", "Ready", "State", "Restarts", "Age"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).style(normal_style).height(1);

        let rows: Vec<Row> = items.iter().map(|pod| make_row(pod)).collect();

        Table::new(rows)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Pods"))
            .highlight_style(selected_style)
            .highlight_symbol(">> ")
            .widths(&[
                Constraint::Min(64),
                Constraint::Min(10),
                Constraint::Min(20),
                Constraint::Min(15),
                Constraint::Min(10),
            ])
    }

    fn on_key(items: &[Arc<Self::Resource>], state: &TableState, key: Key) -> Option<Self::Message>
    where
        <<Self as ListResource>::Resource as kube::Resource>::DynamicType: Hash + Eq,
    {
        match key {
            Key::Char('k') => trigger_kill(items, state),
            _ => None,
        }
    }

    fn process(
        client: Arc<Client>,
        msg: Self::Message,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async {
            match msg {
                Msg::KillPod(pod) => execute_kill(client, &pod).await,
            }
        })
    }
}

fn trigger_kill(pods: &[Arc<Pod>], state: &TableState) -> Option<Msg> {
    let mut pods = pods.to_vec();
    pods.sort_unstable_by(|a, b| a.name().cmp(&b.name()));

    if let Some(pod) = state.selected().and_then(|i| pods.get(i)) {
        Some(Msg::KillPod(pod.clone()))
    } else {
        None
    }
}

fn make_row<'r, 'a>(pod: &'r Pod) -> Row<'a> {
    let mut style = Style::default();

    let name = pod.name();
    let ready = pod.status.as_ref().and_then(make_ready).unwrap_or_default();

    let state = if pod.meta().deletion_timestamp.is_some() {
        PodState::Terminating
    } else {
        pod.status.as_ref().map(make_state).unwrap_or_default()
    };
    let restarts = pod
        .status
        .as_ref()
        .and_then(make_restarts)
        .unwrap_or_else(|| String::from("0"));
    let age = pod
        .creation_timestamp()
        .as_ref()
        .and_then(ago)
        .unwrap_or_default();

    match &state {
        PodState::Pending => {
            style = style.bg(Color::Rgb(128, 0, 128));
        }
        PodState::Error => {
            style = style.bg(Color::Rgb(128, 0, 0)).add_modifier(Modifier::BOLD);
        }
        PodState::CrashLoopBackOff => {
            style = style.bg(Color::Rgb(128, 0, 0));
        }
        PodState::Terminating => {
            style = style.bg(Color::Rgb(128, 128, 0));
        }
        _ => {}
    }

    Row::new(vec![name, ready, state.to_string(), restarts, age]).style(style)
}

#[derive(Debug)]
pub enum Msg {
    KillPod(Arc<Pod>),
}

async fn execute_kill(client: Arc<Client>, pod: &Pod) {
    let result = client
        .run(|context| async move {
            if let Some(namespace) = pod.namespace() {
                let pods: Api<Pod> = Api::namespaced(context.client, &namespace);

                pods.delete(
                    &pod.name(),
                    &DeleteParams::default().preconditions(Preconditions {
                        uid: pod.uid(),
                        ..Default::default()
                    }),
                )
                .await?;
            }
            Ok::<_, anyhow::Error>(())
        })
        .await;

    match result {
        Ok(_) => {
            log::info!("Pod killed");
        }
        Err(err) => {
            log::warn!("Failed to kill pod: {err}");
        }
    }
}
