use crate::app::state::list::ListResource;
use crate::client::Client;
use crate::input::key::Key;
use crate::k8s::ago;
use k8s_openapi::api::apps::v1::Deployment;
use kube::{Api, Resource, ResourceExt};
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use tui::{layout::*, style::*, widgets::*};

pub enum Msg {
    Restart(Arc<Deployment>),
}

pub struct Deployments;

impl ListResource for Deployments {
    type Resource = Deployment;
    type Message = Msg;

    fn render_table<'r, 'a>(items: &'r mut [Arc<Self::Resource>]) -> Table<'a>
    where
        <<Self as ListResource>::Resource as Resource>::DynamicType: Hash + Eq,
    {
        items.sort_unstable_by(|a, b| a.name().cmp(&b.name()));

        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let normal_style = Style::default();
        let header_cells = ["Name", "Ready", "Updated", "Available", "Age"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).style(normal_style).height(1);

        let rows: Vec<Row> = items
            .iter()
            .map(|deployment| Self::make_row(deployment))
            .collect();

        Table::new(rows)
            .header(header)
            .block(Block::default().borders(Borders::ALL).title("Pods"))
            .highlight_style(selected_style)
            .highlight_symbol(">> ")
            .widths(&[
                Constraint::Min(64),
                Constraint::Min(15),
                Constraint::Min(10),
                Constraint::Min(10),
            ])
    }

    fn on_key(items: &[Arc<Self::Resource>], state: &TableState, key: Key) -> Option<Self::Message>
    where
        <<Self as ListResource>::Resource as Resource>::DynamicType: Hash + Eq,
    {
        match key {
            Key::Char('r') => Self::trigger_restart(items, state),
            _ => None,
        }
    }

    fn process(
        client: Arc<Client>,
        msg: Self::Message,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async {
            match msg {
                Msg::Restart(deployment) => {
                    Self::restart(client, &deployment).await;
                }
            }
        })
    }
}

impl Deployments {
    fn trigger_restart(deployments: &[Arc<Deployment>], state: &TableState) -> Option<Msg> {
        let mut deployments = deployments.to_vec();
        deployments.sort_unstable_by(|a, b| a.name().cmp(&b.name()));

        if let Some(deployment) = state.selected().and_then(|i| deployments.get(i)) {
            Some(Msg::Restart(deployment.clone()))
        } else {
            None
        }
    }

    fn make_row<'r, 'a>(deployment: &'r Deployment) -> Row<'a> {
        let style = Style::default();

        let name = deployment.name();
        let (ready, updated, available) = deployment
            .status
            .as_ref()
            .map(|s| {
                (
                    format!(
                        "{}/{}",
                        s.ready_replicas.unwrap_or_default(),
                        s.replicas.unwrap_or_default()
                    ),
                    s.updated_replicas.unwrap_or_default().to_string(),
                    s.available_replicas.unwrap_or_default().to_string(),
                )
            })
            .unwrap_or_default();

        let age = deployment
            .creation_timestamp()
            .as_ref()
            .and_then(ago)
            .unwrap_or_default();

        Row::new(vec![name, ready, updated, available, age]).style(style)
    }

    async fn restart(client: Arc<Client>, deployment: &Deployment) {
        let _ = client
            .run(|ctx| {
                let api: Api<Deployment> = ctx.api_namespaced();
                async move { api.restart(&deployment.name()).await }
            })
            .await;
    }
}
