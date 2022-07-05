use crate::{
    client::Client,
    input::key::Key,
    k8s::Reflector,
    ui::{state::Paging, StateRenderer},
};
use anyhow::anyhow;
use futures::StreamExt;
use k8s_openapi::serde::de::DeserializeOwned;
use kube::runtime::reflector::Store;
use log::log_enabled;
use std::{
    fmt::Debug,
    future::Future,
    hash::Hash,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::{
    spawn,
    sync::mpsc::{channel, Receiver, Sender},
    task::JoinHandle,
};
use tui::{style::*, text::*, widgets::*};

pub trait ListResource: Sized {
    type Resource: kube::Resource
        + Clone
        + Default
        + Debug
        + Send
        + Sync
        + DeserializeOwned
        + 'static;
    type Message: Send + Sync + 'static;

    fn render<SR: StateRenderer>(ctx: &Context<Self>, mut r: SR)
    where
        <<Self as ListResource>::Resource as kube::Resource>::DynamicType: Hash + Eq + Clone,
    {
        let mut state = ctx.state.lock().unwrap();

        match *state {
            State::Loading => {
                let table = Self::render_table(&mut []);
                r.render(table);
            }
            State::List(ref items, ref mut state) => {
                let mut items = items.state();
                let table = { Self::render_table(&mut items) };
                let empty = items.is_empty();

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

    fn render_table<'r, 'a>(items: &'r mut [Arc<Self::Resource>]) -> Table<'a>
    where
        <<Self as ListResource>::Resource as kube::Resource>::DynamicType: Hash + Eq;

    #[allow(unused_variables)]
    fn on_key(items: &[Arc<Self::Resource>], state: &TableState, key: Key) -> Option<Self::Message>
    where
        <<Self as ListResource>::Resource as kube::Resource>::DynamicType: Hash + Eq,
    {
        None
    }

    fn process(client: Arc<Client>, msg: Self::Message)
        -> Pin<Box<dyn Future<Output = ()> + Send>>;
}

struct Runner<R>
where
    R: ListResource,
    <<R as ListResource>::Resource as kube::Resource>::DynamicType: Hash + Eq,
{
    rx: Receiver<R::Message>,
    client: Client,
    ctx: Context<R>,
}

pub struct ListWatcher<R>
where
    R: ListResource,
    <<R as ListResource>::Resource as kube::Resource>::DynamicType: Hash + Eq,
{
    _runner: JoinHandle<()>,
    ctx: Context<R>,
}

pub enum State<K>
where
    K: kube::Resource + 'static,
    K::DynamicType: Hash + Eq,
{
    Loading,
    List(Store<K>, TableState),
    Error(anyhow::Error),
}

pub struct Context<R>
where
    R: ListResource,
    <<R as ListResource>::Resource as kube::Resource>::DynamicType: Hash + Eq,
{
    pub state: Arc<Mutex<State<R::Resource>>>,
    tx: Sender<R::Message>,
}

impl<R> Clone for Context<R>
where
    R: ListResource,
    <<R as ListResource>::Resource as kube::Resource>::DynamicType: Hash + Eq,
{
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            tx: self.tx.clone(),
        }
    }
}

impl<R> ListWatcher<R>
where
    R: ListResource + 'static,
    <<R as ListResource>::Resource as kube::Resource>::DynamicType:
        Hash + Eq + Clone + Default + DeserializeOwned,
{
    pub fn new(client: Client) -> Self {
        let (tx, rx) = channel::<R::Message>(10);

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

        Self {
            _runner: runner,
            ctx,
        }
    }

    pub fn render<SR: StateRenderer>(&self, r: SR) {
        R::render(&self.ctx, r);
    }

    pub async fn on_key(&self, key: Key) {
        self.ctx.on_key(key).await;
    }
}

impl<R: ListResource> Context<R>
where
    <<R as ListResource>::Resource as kube::Resource>::DynamicType: Hash + Eq + Clone,
{
    pub async fn on_key(&self, key: Key) {
        match &mut (*self.state.lock().unwrap()) {
            State::List(items, state) => {
                let items = items.state();
                match key {
                    Key::Down => state.next(items.len()),
                    Key::Up => state.prev(items.len()),
                    _ => {
                        if let Some(msg) = R::on_key(items.as_slice(), state, key) {
                            let _ = self.tx.try_send(msg);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

impl<R> Runner<R>
where
    R: ListResource,
    <<R as ListResource>::Resource as kube::Resource>::DynamicType: Hash + Eq + Clone + Default,
{
    async fn run(mut self) {
        let client = self.client.clone();
        let ctx = self.ctx.clone();

        let reflector = async {
            let mut reflector: Option<Result<Reflector<R::Resource>, anyhow::Error>> = None;

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
            let client = Arc::new(client.clone());
            while let Some(msg) = self.rx.recv().await {
                R::process(client.clone(), msg).await;
            }
        };

        futures::future::select(Box::pin(reflector), Box::pin(receiver)).await;
    }
}
