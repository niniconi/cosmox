use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Result, anyhow};
use cosmox_api::metadata::Metadata;

use url::Url;
use wasmtime::component::Resource;

use crate::plugin_loader::{
    ComponentRunStates, bindings::cosmox::plugin::context as bindings_context,
};

#[derive(Default)]
pub struct MetadataContext {
    pub inner: Option<Arc<Mutex<Metadata<()>>>>,
    pub count: Arc<AtomicU64>,
    caches: HashMap<u64, Arc<Mutex<Metadata<()>>>>,
}

impl MetadataContext {
    pub fn query_by_path(&mut self, path: String) -> Option<Arc<Mutex<Metadata<()>>>> {
        self.inner.as_ref()?;

        let rid = self.inner.clone().unwrap().lock().unwrap().rid;

        let id_list = path
            .split('.')
            .map(|x| x.parse::<u64>().unwrap())
            .collect::<Vec<_>>();

        let id_list = if let Some(first_id) = id_list.first()
            && *first_id == rid
        {
            &id_list[1..]
        } else {
            id_list.as_slice()
        };

        let mut current = self.inner.clone().unwrap();

        for id in id_list {
            match self.caches.get(id) {
                Some(metadata) => current = metadata.clone(),
                None => {
                    let metadata = current.lock().unwrap();
                    let mut tmp = None;

                    if let Some(metadata) = metadata
                        .sub_metadatas
                        .iter()
                        .find(|x| x.lock().unwrap().rid == *id)
                    {
                        tmp = Some(metadata.clone());
                        self.caches.insert(*id, metadata.clone());
                    }

                    drop(metadata);

                    match tmp {
                        Some(metadata) => current = metadata,
                        None => {
                            return None;
                        }
                    }
                }
            }
        }

        Some(current)
    }

    /// Locate the node with `rid` and its parent (`None` if it's the root).
    fn find_with_parent(
        &self,
        rid: u64,
    ) -> Option<(Arc<Mutex<Metadata<()>>>, Option<Arc<Mutex<Metadata<()>>>>)> {
        let root = self.inner.as_ref()?.clone();
        let mut stack = vec![(root, None)];
        while let Some((node, parent)) = stack.pop() {
            let (node_rid, children) = {
                let guard = node.lock().unwrap();
                (guard.rid, guard.sub_metadatas.clone())
            };
            if node_rid == rid {
                return Some((node, parent));
            }
            for child in children {
                stack.push((child, Some(node.clone())));
            }
        }
        None
    }
}

/// Extract the target rid from a `metadata-query` — for `path` this is the
/// last segment of the dot-separated rid chain.
fn metadata_query_rid(query: &bindings_context::MetadataQuery) -> Option<u64> {
    match query {
        bindings_context::MetadataQuery::Id(id) => Some(*id),
        bindings_context::MetadataQuery::Path(path) => path
            .rsplit('.')
            .next()
            .and_then(|segment| segment.parse::<u64>().ok()),
    }
}

impl bindings_context::HostMetadataHandle for ComponentRunStates {
    fn new(&mut self) -> Result<Resource<MetadataContext>> {
        let id = self
            .resource_table
            .push(MetadataContext::default())
            .inspect_err(|err| log::error!("{err}"))?;
        Ok(id)
    }

    fn query(
        &mut self,
        context: Resource<MetadataContext>,
        query: bindings_context::MetadataQuery,
    ) -> Result<Option<Vec<u8>>> {
        let context = self
            .resource_table
            .get_mut(&context)
            .inspect_err(|err| log::error!("{err}"))?;

        if context.caches.is_empty()
            && let Some(root) = &context.inner
        {
            let rid = root.lock().unwrap().rid;
            context.caches.insert(rid, root.clone());
        }

        match query {
            bindings_context::MetadataQuery::Id(id) => Ok(context
                .caches
                .get(&id)
                .map(|x| x.lock().unwrap().binencode().unwrap())),
            bindings_context::MetadataQuery::Path(path) => Ok(context
                .query_by_path(path)
                .map(|x| x.lock().unwrap().binencode().unwrap())),
        }
    }

    fn query_field(
        &mut self,
        context: Resource<MetadataContext>,
        query: bindings_context::MetadataQuery,
        field: String,
    ) -> Result<Option<Vec<u8>>> {
        let context = self
            .resource_table
            .get_mut(&context)
            .inspect_err(|err| log::error!("{err}"))?;

        if context.caches.is_empty()
            && let Some(root) = &context.inner
        {
            let rid = root.lock().unwrap().rid;
            context.caches.insert(rid, root.clone());
        }

        let metadata = match query {
            bindings_context::MetadataQuery::Id(id) => context.caches.get(&id).cloned(),
            bindings_context::MetadataQuery::Path(path) => context.query_by_path(path),
        };

        if let Some(metadata) = metadata {
            let rid = metadata.lock().unwrap().rid;
            context.caches.insert(rid, metadata.clone());

            let config = bincode::config::standard();
            let metadata = metadata.lock().unwrap();
            let result = match field.as_str() {
                "name" => Some(bincode::encode_to_vec(metadata.name.clone(), config)?),
                "description" => Some(bincode::encode_to_vec(
                    metadata.description.clone(),
                    config,
                )?),
                "metadata_type" => Some(bincode::encode_to_vec(
                    metadata.metadata_type.clone(),
                    config,
                )?),
                "url" => Some(bincode::encode_to_vec(metadata.url.clone(), config)?),
                _ => None,
            };
            drop(metadata);
            Ok(result)
        } else {
            Ok(None)
        }
    }

    fn query_all(
        &mut self,
        context: Resource<MetadataContext>,
        query: bindings_context::MetadataQuery,
    ) -> Result<Vec<u64>> {
        let context = self
            .resource_table
            .get_mut(&context)
            .inspect_err(|err| log::error!("{err}"))?;

        if context.caches.is_empty()
            && let Some(root) = &context.inner
        {
            let rid = root.lock().unwrap().rid;
            context.caches.insert(rid, root.clone());
        }

        let childs = match query {
            bindings_context::MetadataQuery::Id(id) => context
                .caches
                .get(&id)
                .map(|x| x.lock().unwrap().sub_metadatas.to_vec()),
            bindings_context::MetadataQuery::Path(path) => context
                .query_by_path(path)
                .map(|x| x.lock().unwrap().sub_metadatas.to_vec()),
        };

        match childs {
            Some(childs) => {
                let childs = childs
                    .iter()
                    .map(|x| {
                        let rid = x.lock().unwrap().rid;
                        context.caches.insert(rid, x.clone());
                        rid
                    })
                    .collect::<Vec<_>>();

                Ok(childs)
            }
            None => Ok(vec![]),
        }
    }

    fn move_(
        &mut self,
        context: Resource<MetadataContext>,
        query: bindings_context::MetadataQuery,
        target: bindings_context::MetadataQuery,
    ) -> Result<()> {
        log::trace!("metadata context move {query:?} to {target:?}");
        let context = self
            .resource_table
            .get_mut(&context)
            .inspect_err(|err| log::error!("{err}"))?;

        let node_rid =
            metadata_query_rid(&query).ok_or_else(|| anyhow!("invalid query {query:?}"))?;
        let (node, old_parent) = context
            .find_with_parent(node_rid)
            .ok_or_else(|| anyhow!("metadata {node_rid} not found"))?;

        let target_parent = match &target {
            bindings_context::MetadataQuery::Id(id) => context.caches.get(id).cloned(),
            bindings_context::MetadataQuery::Path(path) => context.query_by_path(path.clone()),
        }
        .ok_or_else(|| anyhow!("target parent {target:?} not found"))?;

        let Some(old_parent) = old_parent else {
            return Err(anyhow!("cannot move root metadata {node_rid}"));
        };

        // Reject moves that would create a cycle: target must not be the node
        // itself nor any of its descendants.
        if Arc::ptr_eq(&old_parent, &target_parent) {
            return Ok(());
        }
        let target_rid = target_parent.lock().unwrap().rid;
        if target_rid == node_rid {
            return Err(anyhow!("cannot move metadata {node_rid} into itself"));
        }
        let mut stack = vec![node.clone()];
        while let Some(candidate) = stack.pop() {
            let children = {
                let guard = candidate.lock().unwrap();
                guard.sub_metadatas.clone()
            };
            for child in children {
                if child.lock().unwrap().rid == target_rid {
                    return Err(anyhow!(
                        "cannot move metadata {node_rid} into its own subtree"
                    ));
                }
                stack.push(child);
            }
        }

        old_parent
            .lock()
            .unwrap()
            .sub_metadatas
            .retain(|x| !Arc::ptr_eq(x, &node));

        target_parent
            .lock()
            .unwrap()
            .sub_metadatas
            .push(node.clone());

        context.caches.insert(target_rid, target_parent.clone());
        context.caches.insert(node_rid, node);

        Ok(())
    }

    fn insert(
        &mut self,
        context: Resource<MetadataContext>,
        query: bindings_context::MetadataQuery,
        data: Vec<u8>,
    ) -> Result<u64> {
        log::trace!("metadata context insert to {query:?}, data: {data:?}");
        let context = self.resource_table.get_mut(&context)?;
        let parent_metadata = match &query {
            bindings_context::MetadataQuery::Id(id) => context.caches.get(id).cloned(),
            bindings_context::MetadataQuery::Path(path) => context.query_by_path(path.clone()),
        };
        match parent_metadata {
            Some(parent_metadata) => {
                let metadata_data: Arc<Mutex<Metadata<()>>> =
                    Metadata::bindecode_set_id(data, context.count.fetch_add(1, Ordering::Relaxed))
                        .unwrap();

                parent_metadata
                    .lock()
                    .unwrap()
                    .sub_metadatas
                    .push(metadata_data.clone());
                let new_rid = metadata_data.lock().unwrap().rid;
                context.caches.insert(new_rid, metadata_data);

                Ok(new_rid)
            }
            None => Err(anyhow!("parent metadata {query:?} not found")),
        }
    }

    fn delete(
        &mut self,
        context: Resource<MetadataContext>,
        query: bindings_context::MetadataQuery,
    ) -> Result<()> {
        log::trace!("metadata context delete {query:?}");
        let context = self
            .resource_table
            .get_mut(&context)
            .inspect_err(|err| log::error!("{err}"))?;

        let rid = metadata_query_rid(&query).ok_or_else(|| anyhow!("invalid query {query:?}"))?;
        let (node, parent) = context
            .find_with_parent(rid)
            .ok_or_else(|| anyhow!("metadata {rid} not found"))?;
        let Some(parent) = parent else {
            return Err(anyhow!("cannot delete root metadata {rid}"));
        };

        parent
            .lock()
            .unwrap()
            .sub_metadatas
            .retain(|x| !Arc::ptr_eq(x, &node));

        // Drop cache entries for the deleted node and its whole subtree.
        let mut stack = vec![node];
        while let Some(candidate) = stack.pop() {
            let (candidate_rid, children) = {
                let guard = candidate.lock().unwrap();
                (guard.rid, guard.sub_metadatas.clone())
            };
            context.caches.remove(&candidate_rid);
            stack.extend(children);
        }

        Ok(())
    }

    fn modify(
        &mut self,
        context: Resource<MetadataContext>,
        query: bindings_context::MetadataQuery,
        field: String,
        data: Vec<u8>,
    ) -> Result<()> {
        log::trace!("metadata context modify field {field}, data: {data:?}");
        let context = self.resource_table.get_mut(&context)?;
        let node = match query {
            bindings_context::MetadataQuery::Id(id) => context.caches.get(&id).cloned(),
            bindings_context::MetadataQuery::Path(path) => context.query_by_path(path),
        };

        if let Some(metadata) = node {
            let mut metadata = metadata.lock().unwrap();
            let config = bincode::config::standard();
            match field.as_str() {
                "name" => {
                    metadata.name = bincode::decode_from_slice(&data, config)?.0;
                }
                "description" => {
                    metadata.description = bincode::decode_from_slice(&data, config)?.0;
                }
                "origin_name" => {
                    metadata.origin_name = bincode::decode_from_slice(&data, config)?.0;
                }
                "origin" => {
                    metadata.origin = bincode::decode_from_slice(&data, config)?.0;
                }
                s if s.starts_with(":") => {
                    if let Some(key) = s.get(1..) {
                        metadata.extend.insert(
                            key.to_string(),
                            bincode::decode_from_slice(&data, config)?.0,
                        );
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn drop(&mut self, context: Resource<MetadataContext>) -> wasmtime::Result<()> {
        self.resource_table
            .delete(context)
            .inspect_err(|err| log::error!("{err}"))?;
        Ok(())
    }
}

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
                if field == "cover_file_map_id"
                    || field == "data_file_map_id"
                    || field.starts_with(':')
                {
                    let mut path_mapping_temp = context.path_mapping_temp.lock().unwrap();
                    if let Some(path_mapping_temp) = path_mapping_temp.get_mut(&id) {
                        path_mapping_temp.push((field, url));
                    } else {
                        path_mapping_temp.insert(id, vec![(field, url)]);
                    }

                    // TODO
                    // anyhow::anyhow!(
                    //   "Expected one of: `cover_file_map_id`, `data_file_map_id`. or start with `:`"
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
