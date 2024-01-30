use crate::Args;
use k8s_openapi::NamespaceResourceScope;
use kube::{
    config::{KubeConfigOptions, KubeconfigError},
    Api, Resource,
};
use std::future::Future;

#[derive(Debug, thiserror::Error)]
pub enum RunError<E> {
    #[error("Failed to evaluate configuration: {0}")]
    Config(#[from] KubeconfigError),
    #[error("Failed to create client: {0}")]
    Kube(#[from] kube::Error),
    #[error("Failed to create client: {0}")]
    Operation(#[source] E),
}

#[derive(Clone)]
pub struct Client {
    args: Args,
}

impl Client {
    pub fn new(args: Args) -> Self {
        Self { args }
    }

    pub async fn run<F, Fut, R, E>(&self, f: F) -> Result<R, RunError<E>>
    where
        F: FnOnce(Context) -> Fut,
        Fut: Future<Output = Result<R, E>>,
    {
        // right now, we just create a new client every time. later on, we should cache
        // and invalidate the cache when an operation fails
        let config = kube::Config::from_kubeconfig(&KubeConfigOptions {
            context: self.args.context.clone(),
            ..Default::default()
        })
        .await?;
        let client = kube::Client::try_from(config)?;

        let context = Context {
            client,
            args: &self.args,
        };

        match f(context).await {
            Ok(result) => Ok(result),
            Err(err) => Err(RunError::Operation(err)),
        }
    }
}

#[derive(Clone)]
pub struct Context<'c> {
    pub args: &'c Args,
    pub client: kube::Client,
}

impl Context<'_> {
    pub fn api_namespaced<K>(self) -> Api<K>
    where
        K: Resource<Scope = NamespaceResourceScope>,
        <K as Resource>::DynamicType: Default,
    {
        match &self.args.namespace {
            Some(namespace) => Api::namespaced(self.client, &namespace),
            None => Api::default_namespaced(self.client),
        }
    }
}
