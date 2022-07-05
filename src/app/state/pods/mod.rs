mod data;

use data::*;

use crate::{
    client::Client,
    input::key::Key,
    ui::{state::Paging, StateRenderer},
};
use anyhow::anyhow;
use futures::{Stream, StreamExt};
use k8s_openapi::{api::core::v1::Pod, serde::de::DeserializeOwned};
use kube::{
    api::{DeleteParams, ListParams, Preconditions},
    runtime::{
        reflector::{self, reflector, Store},
        watcher,
    },
    Api, Resource, ResourceExt,
};
use log::log_enabled;
use std::{
    convert::Infallible,
    fmt::Debug,
    hash::Hash,
    ops::Deref,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::{
    spawn,
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
};
use tui::{layout::*, style::*, text::*, widgets::*};

pub struct Pods {
    _runner: JoinHandle<()>,
    ctx: Context,
}

pub enum State {
    Loading,
    List(Store<Pod>, TableState),
    Error(anyhow::Error),
}

#[derive(Clone)]
struct Context {
    state: Arc<Mutex<State>>,
    tx: Sender<Msg>,
}

struct Runner {
    rx: Receiver<Msg>,
    client: Client,
    ctx: Context,
}

#[derive(Debug)]
enum Msg {
    KillPod(Arc<Pod>),
}

impl Pods {
    pub fn new(client: Client) -> Self {
        let (tx, rx) = channel::<Msg>(10);

        let ctx = Context {
            tx,
            state: Arc::new(Mutex::new(State::Loading)),
        };

        let runner = Runner {
            rx,
            client,
            ctx: ctx.clone(),
        };

        let runner = spawn(async move {
            runner.run().await;
        });
        Pods {
            _runner: runner,
            ctx,
        }
    }

    pub fn render<R: StateRenderer>(&self, r: R) {
        self.ctx.render(r);
    }

    pub async fn on_key(&self, key: Key) {
        self.ctx.on_key(key).await;
    }
}

impl Context {
    pub async fn on_key(&self, key: Key) {
        match &mut (*self.state.lock().unwrap()) {
            State::List(pods, state) => {
                let pods = pods.state();
                match key {
                    Key::Down => state.next(pods.len()),
                    Key::Up => state.prev(pods.len()),
                    Key::Char('k') => self.trigger_kill(pods.as_slice(), state).await,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    async fn trigger_kill(&self, pods: &[Arc<Pod>], state: &TableState) {
        if let Some(pod) = state.selected().and_then(|i| pods.get(i)) {
            let _ = self.tx.try_send(Msg::KillPod(pod.clone()));
        }
    }

    fn render_table(pods: &[Arc<Pod>]) -> (Table, bool) {
        let selected_style = Style::default().add_modifier(Modifier::REVERSED);
        let normal_style = Style::default();
        let header_cells = ["Name", "Ready", "State", "Restarts", "Age"]
            .iter()
            .map(|h| Cell::from(*h).style(Style::default().add_modifier(Modifier::BOLD)));
        let header = Row::new(header_cells).style(normal_style).height(1);

        let rows: Vec<Row> = pods.iter().map(|pod| Self::make_row(pod)).collect();

        let empty = rows.is_empty();

        (
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
                ]),
            empty,
        )
    }

    pub fn render<R: StateRenderer>(&self, mut r: R) {
        let mut state = self.state.lock().unwrap();

        match *state {
            State::Loading => {
                let (table, _) = Self::render_table(&[]);
                r.render(table);
            }
            State::List(ref pods, ref mut state) => {
                let pods = pods.state();
                let (table, empty) = Self::render_table(&pods);

                if state.selected().is_none() && !empty {
                    state.select(Some(0));
                }

                r.render_stateful(table, state);
            }
            State::Error(ref err) => {
                let err = err.to_string();
                let w = Paragraph::new(err)
                    .style(Style::default().bg(Color::Rgb(128, 0, 0)))
                    .block(
                        Block::default()
                            .title(Span::styled(
                                "Error",
                                Style::default().add_modifier(Modifier::BOLD),
                            ))
                            .borders(Borders::ALL),
                    );
                r.render(w);
            }
        }
    }

    fn make_row(pod: &Pod) -> Row {
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
}

impl Runner {
    async fn run(mut self) {
        let client = self.client.clone();
        let ctx = self.ctx.clone();

        let reflector = async {
            let mut reflector: Option<Result<Reflector<Pod>, anyhow::Error>> = None;

            'outer: loop {
                match reflector {
                    None => {
                        *ctx.state.lock().unwrap() = State::Loading;
                        // Create
                        reflector = Some(Reflector::new(&client).await);
                    }
                    Some(Err(err)) => {
                        // set error
                        {
                            *ctx.state.lock().unwrap() = State::Error(anyhow!(err));
                        }
                        // create
                        let r = Reflector::new(&client).await;
                        log::warn!("Created new reflector - ok: {}", r.is_ok());
                        reflector = Some(r);
                    }
                    Some(Ok(mut r)) => {
                        // set store
                        {
                            *ctx.state.lock().unwrap() =
                                State::List(r.reader.clone(), Default::default());
                        }
                        // run
                        while let Some(evt) = r.stream.next().await {
                            if log_enabled!(log::Level::Info) {
                                let m = format!("{evt:?}");
                                log::info!("{}", &m[0..90]);
                            }
                            match evt {
                                Ok(_) => {}
                                Err(err) => {
                                    log::warn!("Watch error: {err}");
                                    reflector = Some(Err(anyhow!(err)));
                                    continue 'outer;
                                }
                            }
                        }
                        log::warn!("Stream closed");
                        reflector = Some(Err(anyhow!("Stream closed")));
                    }
                }
            }
        };

        let receiver = async {
            while let Some(msg) = self.rx.recv().await {
                match msg {
                    Msg::KillPod(pod) => self.execute_kill(&pod).await,
                }
            }
        };

        futures::future::select(Box::pin(reflector), Box::pin(receiver)).await;
    }

    async fn execute_kill(&self, pod: &Pod) {
        let result = self
            .client
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
}

pub struct Reflector<K>
where
    K: Resource + 'static,
    K::DynamicType: Hash + Eq,
{
    reader: Store<K>,
    stream: Pin<Box<dyn Stream<Item = watcher::Result<watcher::Event<K>>> + Send>>,
}

impl<K> Reflector<K>
where
    K: Resource + Debug + Send + Sync + DeserializeOwned + Clone + 'static,
    K::DynamicType: Clone + Default + Hash + Eq,
{
    pub async fn new(client: &Client) -> anyhow::Result<Reflector<K>> {
        Ok(client
            .run(|context| {
                let pods: Api<K> = context.api_namespaced();
                async {
                    let (reader, writer) = reflector::store();
                    let lp = ListParams::default();
                    let stream = Box::pin(reflector(writer, watcher(pods, lp)));
                    Ok::<_, Infallible>(Reflector { reader, stream })
                }
            })
            .await?)
    }
}

impl<K> Deref for Reflector<K>
where
    K: Resource + Debug + Send + DeserializeOwned + Clone + 'static,
    K::DynamicType: Clone + Default + Hash + Eq,
{
    type Target = Store<K>;

    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}
