use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::Result;

use wasmtime::component::Resource;

use crate::plugin_loader::{
    ComponentRunStates, bindings::cosmox::plugin::context as bindings_context,
};

pub type TagContextTemp = Arc<Mutex<HashMap<u64, Vec<(String, String)>>>>;
#[derive(Default)]
pub struct TagContext {
    pub tag_temp: TagContextTemp,
}

impl bindings_context::HostTagHandle for ComponentRunStates {
    fn new(&mut self) -> Result<Resource<TagContext>> {
        let id = self
            .resource_table
            .push(TagContext::default())
            .inspect_err(|err| log::error!("{err}"))?;
        Ok(id)
    }
    fn add_tag(
        &mut self,
        context: Resource<TagContext>,
        rid: u64,
        label: (String, String),
    ) -> Result<Result<(), String>> {
        let context = self
            .resource_table
            .get_mut(&context)
            .inspect_err(|err| log::error!("{err}"))?;

        let mut tag_temp = context.tag_temp.lock().unwrap();
        if let Some(tag_temp) = tag_temp.get_mut(&rid) {
            tag_temp.push(label);
        } else {
            tag_temp.insert(rid, vec![label]);
        }

        Ok(Ok(()))
    }

    fn add_tags(
        &mut self,
        context: Resource<TagContext>,
        rid: u64,
        labels: Vec<(String, String)>,
    ) -> Result<Result<(), String>> {
        log::debug!("insert tags{labels:?}");
        let context = self
            .resource_table
            .get_mut(&context)
            .inspect_err(|err| log::error!("{err}"))?;

        let mut tag_temp = context.tag_temp.lock().unwrap();
        let tag_temp = match tag_temp.get_mut(&rid) {
            Some(tag_temp) => tag_temp,
            None => {
                tag_temp.insert(rid, vec![]);
                tag_temp.get_mut(&rid).unwrap()
            }
        };

        for label in labels {
            tag_temp.push(label);
        }
        Ok(Ok(()))
    }

    fn delete_tag(
        &mut self,
        context: wasmtime::component::Resource<TagContext>,
        tid: u64,
    ) -> Result<Result<(), String>> {
        log::debug!("delete tags {tid}");
        let context = self
            .resource_table
            .get_mut(&context)
            .inspect_err(|err| log::error!("{err}"))?;

        context.tag_temp.lock().unwrap().remove(&tid);
        Ok(Ok(()))
    }
    fn drop(&mut self, context: Resource<TagContext>) -> wasmtime::Result<()> {
        self.resource_table
            .delete(context)
            .inspect_err(|err| log::error!("{err}"))?;
        Ok(())
    }
}
