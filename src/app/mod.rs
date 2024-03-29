use crate::app::state::list::ListWatcher;
use crate::app::state::AppState;
use crate::client::Client;
use crate::input::key::Key;
use crate::Args;

pub mod state;
pub mod ui;

#[derive(Debug, PartialEq, Eq)]
pub enum AppReturn {
    Exit,
    Continue,
}

pub struct App {
    state: AppState,
    client: Client,
    args: Args,
    global: Global,
}

#[derive(Default)]
pub struct Global {
    pub logs: bool,
    pub help: bool,
}

impl App {
    pub fn new(args: Args) -> Self {
        let client = Client::new(args.clone());
        Self {
            state: AppState::Pods(ListWatcher::new(client.clone())),
            client,
            args,
            global: Default::default(),
        }
    }

    /// Handle a user action
    pub async fn do_action(&mut self, key: Key) -> AppReturn {
        log::debug!("Key: {key:?}");

        match key {
            Key::Ctrl('c') | Key::Char('q') => return AppReturn::Exit,
            Key::Esc => {
                if self.global.help {
                    self.global.help = false;
                } else {
                    return AppReturn::Exit;
                }
            }
            Key::Char('d') => {
                self.state = AppState::Deployments(ListWatcher::new(self.client.clone()))
            }
            Key::Char('p') => self.state = AppState::Pods(ListWatcher::new(self.client.clone())),
            Key::Char('l') => self.global.logs = !self.global.logs,
            Key::Char('h') | Key::Char('?') => self.global.help = !self.global.help,
            Key::Left => self.prev(),
            Key::Right => self.next(),
            _ => {
                self.state.on_key(key).await;
            }
        }
        AppReturn::Continue
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn global(&self) -> &Global {
        &self.global
    }

    pub fn prev(&mut self) {
        self.state = match &self.state {
            AppState::Initializing => AppState::Initializing,
            AppState::Pods(_) => AppState::Deployments(ListWatcher::new(self.client.clone())),
            AppState::Deployments(_) => AppState::Pods(ListWatcher::new(self.client.clone())),
        }
    }

    pub fn next(&mut self) {
        self.state = match &self.state {
            AppState::Initializing => AppState::Initializing,
            AppState::Pods(_) => AppState::Deployments(ListWatcher::new(self.client.clone())),
            AppState::Deployments(_) => AppState::Pods(ListWatcher::new(self.client.clone())),
        }
    }
}
