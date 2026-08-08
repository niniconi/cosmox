use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::Result;

use url::Url;
use wasmtime::component::Resource;

use crate::plugin_loader::{
    ComponentRunStates, bindings::cosmox::plugin::context as bindings_context,
};

pub type PathMappingContextTemp = Arc<Mutex<HashMap<u64, Vec<(String, Url)>>>>;

#[derive(Default)]
pub struct PathMappingContext {
    pub path_mapping_temp: PathMappingContextTemp,
}

impl bindings_context::HostPathMappingHandle for ComponentRunStates {
    fn new(&mut self) -> Result<Resource<PathMappingContext>> {
        let id = self
            .resource_table
            .push(PathMappingContext::default())
            .inspect_err(|err| log::error!("{err}"))?;
        Ok(id)
    }

    fn push(
        &mut self,
        context: Resource<PathMappingContext>,
        id: u64,
        field: String,
        link: String,
    ) -> wasmtime::Result<Result<(), bindings_context::PathMappingHandleError>> {
        let context = self
            .resource_table
            .get_mut(&context)
            .inspect_err(|err| log::error!("{err}"))?;

        let url = Url::parse(link.as_str())
            .inspect_err(|err| log::error!("{err}"))
            .map_err(|err| {
                bindings_context::PathMappingHandleError::InvalidFormat(err.to_string())
            });

        let res = match url {
            Ok(url) => {
                if field == "cover_file_map_id" || field == "data_file_map_id" {
                    let mut path_mapping_temp = context.path_mapping_temp.lock().unwrap();
                    if let Some(path_mapping_temp) = path_mapping_temp.get_mut(&id) {
                        path_mapping_temp.push((field, url));
                    } else {
                        path_mapping_temp.insert(id, vec![(field, url)]);
                    }

                    // TODO
                    // anyhow::anyhow!(
                    //   "Expected one of: `cover_file_map_id` or `data_file_map_id`."
                    // )
                }
                Ok(())
            }
            Err(err) => Err(err),
        };
        Ok(res)
    }
    fn drop(
        &mut self,
        context: Resource<bindings_context::PathMappingHandle>,
    ) -> wasmtime::Result<()> {
        self.resource_table
            .delete(context)
            .inspect_err(|err| log::error!("{err}"))?;
        Ok(())
    }
}
