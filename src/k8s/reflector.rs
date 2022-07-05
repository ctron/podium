use crate::client::Client;
use futures::Stream;
use k8s_openapi::serde::de::DeserializeOwned;
use kube::{
    api::ListParams,
    runtime::{
        reflector::{self, reflector, Store},
        watcher,
    },
    Api, Resource,
};
use std::{convert::Infallible, fmt::Debug, hash::Hash, ops::Deref, pin::Pin};

pub struct Reflector<K>
where
    K: Resource + 'static,
    K::DynamicType: Hash + Eq,
{
    pub reader: Store<K>,
    pub stream: Pin<Box<dyn Stream<Item = watcher::Result<watcher::Event<K>>> + Send>>,
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
