use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    api::{Patch, PatchParams},
    Api,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::fmt::Debug;

pub trait Scalable {}

pub trait Scale {
    type Resource;

    async fn replicas(&self, name: &str, replicas: i32) -> Result<Self::Resource, kube::Error>;
}

impl<S> Scale for Api<S>
where
    S: Scalable + Clone + DeserializeOwned + Debug,
{
    type Resource = S;

    async fn replicas(&self, name: &str, replicas: i32) -> Result<Self::Resource, kube::Error> {
        let pp = PatchParams::default();
        self.patch(
            name,
            &pp,
            &Patch::Strategic(json!({"spec":{"replicas": replicas}})),
        )
        .await
    }
}

impl Scalable for Deployment {}
