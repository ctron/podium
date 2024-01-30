use crate::app::state::list::ListResource;
use crate::client::Client;
use crate::input::key::Key;
use crate::k8s::{ago, Scale};
use k8s_openapi::api::apps::v1::Deployment;
use kube::{Api, Resource, ResourceExt};
use ratatui::{layout::*, style::*, widgets::*};
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;

pub enum Msg {
    Restart(Arc<Deployment>),
    ScaleUp(Arc<Deployment>),
    ScaleDown(Arc<Deployment>),
}

pub struct Deployments;

impl ListResource for Deployments {
    type Resource = Deployment;
    type Message = Msg;

    fn render_table<'r, 'a>(items: &'r mut [Arc<Self::Resource>]) -> Table<'a>
    where
        <<Self as ListResource>::Resource as Resource>::DynamicType: Hash + Eq,
    {
        items.sort_unstable_by(|a, b| a.name_any().cmp(&b.name_any()));

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

        Table::new(
            rows,
            [
                Constraint::Min(64),
                Constraint::Min(15),
                Constraint::Min(10),
                Constraint::Min(10),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Deployments"))
        .highlight_style(selected_style)
        .highlight_symbol(">> ")
    }

    fn on_key(items: &[Arc<Self::Resource>], state: &TableState, key: Key) -> Option<Self::Message>
    where
        <<Self as ListResource>::Resource as Resource>::DynamicType: Hash + Eq,
    {
        match key {
            Key::Char('r') => Self::with_selection(items, state, Msg::Restart),
            Key::Char('+') => Self::with_selection(items, state, Msg::ScaleUp),
            Key::Char('-') => Self::with_selection(items, state, Msg::ScaleDown),
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
                Msg::ScaleUp(deployment) => {
                    Self::scale(client, &deployment, 1).await;
                }
                Msg::ScaleDown(deployment) => {
                    Self::scale(client, &deployment, -1).await;
                }
            }
        })
    }
}

impl Deployments {
    fn with_selection<F, I>(
        deployments: &[Arc<Deployment>],
        state: &TableState,
        f: F,
    ) -> Option<Msg>
    where
        F: FnOnce(Arc<Deployment>) -> I,
        I: Into<Option<Msg>>,
    {
        let mut deployments = deployments.to_vec();
        deployments.sort_unstable_by(|a, b| a.name_any().cmp(&b.name_any()));

        if let Some(deployment) = state.selected().and_then(|i| deployments.get(i)) {
            f(deployment.clone()).into()
        } else {
            None
        }
    }

    fn make_row<'r, 'a>(deployment: &'r Deployment) -> Row<'a> {
        let mut style = Style::default();

        let name = deployment.name_any();

        let (ready, updated, available) = deployment
            .status
            .as_ref()
            .map(|s| {
                (
                    (
                        s.ready_replicas.unwrap_or_default(),
                        s.replicas.unwrap_or_default(),
                    ),
                    s.updated_replicas.unwrap_or_default(),
                    s.available_replicas.unwrap_or_default(),
                )
            })
            .unwrap_or_default();

        let age = deployment
            .creation_timestamp()
            .as_ref()
            .and_then(ago)
            .unwrap_or_default();

        if ready.0 == 0 {
            style.fg = Some(Color::Red);
        } else if ready.0 < ready.1 {
            style.fg = Some(Color::Yellow);
        }

        Row::new(vec![
            name,
            format!("{}/{}", ready.0, ready.1),
            updated.to_string(),
            available.to_string(),
            age,
        ])
        .style(style)
    }

    async fn restart(client: Arc<Client>, deployment: &Deployment) {
        let _ = client
            .run(|ctx| {
                let api: Api<Deployment> = ctx.api_namespaced();
                async move { api.restart(&deployment.name_any()).await }
            })
            .await;
    }

    async fn scale(client: Arc<Client>, deployment: &Deployment, amount: i32) {
        let _ = client
            .run(|ctx| {
                let api: Api<Deployment> = ctx.api_namespaced();
                async move {
                    let current: i32 = deployment
                        .spec
                        .as_ref()
                        .and_then(|s| s.replicas)
                        .unwrap_or_default();

                    let replicas = current.saturating_add(amount);
                    if replicas != current {
                        api.replicas(&deployment.name_any(), replicas)
                            .await
                            .map(|_| ())
                    } else {
                        Ok(())
                    }
                }
            })
            .await;
    }
}
