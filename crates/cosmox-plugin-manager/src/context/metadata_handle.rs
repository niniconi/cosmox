use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::{Result, anyhow};
use cosmox_api::metadata::{Metadata, MetadataNode};

use wasmtime::component::Resource;

use crate::plugin_loader::{
    ComponentRunStates, bindings::cosmox::plugin::context as bindings_context,
};

/// A cached node together with its parent (`None` for the root).
///
/// The `parent` pointer turns `find_with_parent` into an O(1) index lookup
/// instead of a full-tree BFS on every move/delete.
struct CachedNode {
    node: MetadataNode,
    parent: Option<MetadataNode>,
}

#[derive(Default)]
pub struct MetadataContext {
    pub inner: Option<MetadataNode>,
    pub count: Arc<AtomicU64>,
    caches: HashMap<u64, CachedNode>,
}

impl MetadataContext {
    /// Find `segment` among the direct children of `current` by name,
    /// warming the cache for the resolved rid.
    fn find_child(&mut self, current: &MetadataNode, segment: &str) -> Option<MetadataNode> {
        let children = {
            let guard = current.lock().unwrap();
            guard.sub_metadatas.clone()
        };
        let child = children
            .iter()
            .find(|x| x.lock().unwrap().name == segment)?
            .clone();
        let rid = child.lock().unwrap().rid;
        self.caches.insert(
            rid,
            CachedNode {
                node: child.clone(),
                parent: Some(current.clone()),
            },
        );
        Some(child)
    }

    /// Resolve a `/`-separated name chain against the current tree, starting
    /// from the root's direct children.
    pub fn query_by_path(&mut self, path: String) -> Option<MetadataNode> {
        let root = self.inner.as_ref()?.clone();
        let root_rid = root.lock().unwrap().rid;
        self.caches.insert(
            root_rid,
            CachedNode {
                node: root.clone(),
                parent: None,
            },
        );

        let mut current = root;

        for segment in path.split('/').filter(|s| !s.is_empty()) {
            current = self.find_child(&current, segment)?;
        }

        Some(current)
    }

    /// Resolve the node addressed by `query` — `path` is resolved as a name
    /// chain; `id` falls back to a tree lookup when the cache is cold.
    fn resolve(&mut self, query: &bindings_context::MetadataQuery) -> Option<MetadataNode> {
        match query {
            bindings_context::MetadataQuery::Id(id) => {
                if let Some(entry) = self.caches.get(id) {
                    Some(entry.node.clone())
                } else {
                    self.find_with_parent(*id).map(|(node, _)| node)
                }
            }
            bindings_context::MetadataQuery::Path(path) => self.query_by_path(path.clone()),
        }
    }

    /// Locate the node with `rid` and its parent (`None` if it's the root).
    fn find_with_parent(&mut self, rid: u64) -> Option<(MetadataNode, Option<MetadataNode>)> {
        if let Some(entry) = self.caches.get(&rid) {
            return Some((entry.node.clone(), entry.parent.clone()));
        }
        let root = self.inner.as_ref()?.clone();
        let mut stack = vec![(root, None)];
        while let Some((node, parent)) = stack.pop() {
            let (node_rid, children) = {
                let guard = node.lock().unwrap();
                (guard.rid, guard.sub_metadatas.clone())
            };
            if node_rid == rid {
                self.caches.insert(
                    rid,
                    CachedNode {
                        node: node.clone(),
                        parent: parent.clone(),
                    },
                );
                return Some((node, parent));
            }
            for child in children {
                stack.push((child, Some(node.clone())));
            }
        }
        None
    }
}

/// Resolve the rid addressed by `query` — `path` is a `/`-separated name
/// chain resolved against the current tree.
fn metadata_query_rid(
    context: &mut MetadataContext,
    query: &bindings_context::MetadataQuery,
) -> Option<u64> {
    match query {
        bindings_context::MetadataQuery::Id(id) => Some(*id),
        bindings_context::MetadataQuery::Path(path) => context
            .query_by_path(path.clone())
            .map(|node| node.lock().unwrap().rid),
    }
}

/// Move `query` under `target`, rejecting cycles. Shared by the single
/// `move` and the batch `move_by_ids` host methods.
fn move_node(
    context: &mut MetadataContext,
    query: &bindings_context::MetadataQuery,
    target: &bindings_context::MetadataQuery,
) -> Result<()> {
    let node_rid =
        metadata_query_rid(context, query).ok_or_else(|| anyhow!("invalid query {query:?}"))?;
    let (node, old_parent) = context
        .find_with_parent(node_rid)
        .ok_or_else(|| anyhow!("metadata {node_rid} not found"))?;

    let target_parent = context
        .resolve(target)
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

    // Re-index `node` under its new parent; `target_parent` was already
    // indexed by `resolve` above.
    context.caches.insert(
        node_rid,
        CachedNode {
            node: node.clone(),
            parent: Some(target_parent.clone()),
        },
    );

    Ok(())
}

/// Detach `rid` from its parent and drop cache entries for its subtree.
/// Shared by the single `delete` and the batch `delete_by_ids` host methods.
fn delete_node(context: &mut MetadataContext, rid: u64) -> Result<()> {
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

        Ok(context
            .resolve(&query)
            .map(|x| x.lock().unwrap().binencode().unwrap()))
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

        let metadata = context.resolve(&query);

        if let Some(metadata) = metadata {
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
                // Extend keys are addressed verbatim (`EXTEND_KEY:field`,
                // e.g. `anime:season_number`); a leading `:` prefix is not
                // accepted and resolves to nothing.
                _ => metadata
                    .extend
                    .get(&field)
                    .map(|value| bincode::encode_to_vec(value, config))
                    .transpose()?,
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

        match context.resolve(&query) {
            Some(node) => {
                let children = {
                    let guard = node.lock().unwrap();
                    guard.sub_metadatas.clone()
                };
                let childs = children
                    .iter()
                    .map(|child| {
                        let rid = child.lock().unwrap().rid;
                        context.caches.insert(
                            rid,
                            CachedNode {
                                node: child.clone(),
                                parent: Some(node.clone()),
                            },
                        );
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
        move_node(context, &query, &target)
    }

    fn insert(
        &mut self,
        context: Resource<MetadataContext>,
        query: bindings_context::MetadataQuery,
        data: Vec<u8>,
    ) -> Result<u64> {
        log::trace!("metadata context insert to {query:?}, data: {data:?}");
        let context = self.resource_table.get_mut(&context)?;
        let parent_metadata = context.resolve(&query);
        match parent_metadata {
            Some(parent_metadata) => {
                let metadata_data: MetadataNode =
                    Metadata::bindecode_set_id(data, context.count.fetch_add(1, Ordering::Relaxed))
                        .unwrap();

                parent_metadata
                    .lock()
                    .unwrap()
                    .sub_metadatas
                    .push(metadata_data.clone());
                let new_rid = metadata_data.lock().unwrap().rid;
                context.caches.insert(
                    new_rid,
                    CachedNode {
                        node: metadata_data.clone(),
                        parent: Some(parent_metadata.clone()),
                    },
                );

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

        let rid = metadata_query_rid(context, &query)
            .ok_or_else(|| anyhow!("invalid query {query:?}"))?;
        delete_node(context, rid)
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
        let node = context.resolve(&query);

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
                _ => {
                    metadata
                        .extend
                        .insert(field.clone(), bincode::decode_from_slice(&data, config)?.0);
                }
            }
        }
        Ok(())
    }

    fn write_extend(
        &mut self,
        context: Resource<MetadataContext>,
        query: bindings_context::MetadataQuery,
        pairs: Vec<(String, String)>,
    ) -> Result<()> {
        log::trace!("metadata context write_extend on {query:?}: {pairs:?}");
        let context = self.resource_table.get_mut(&context)?;
        let node = context.resolve(&query);
        if let Some(metadata) = node {
            let mut metadata = metadata.lock().unwrap();
            for (k, v) in pairs {
                metadata.extend.insert(k, v);
            }
        }
        Ok(())
    }

    fn move_by_ids(
        &mut self,
        context: Resource<MetadataContext>,
        target: bindings_context::MetadataQuery,
        ids: Vec<u64>,
    ) -> Result<()> {
        log::trace!("metadata context move_by_ids {ids:?} to {target:?}");
        let context = self
            .resource_table
            .get_mut(&context)
            .inspect_err(|err| log::error!("{err}"))?;
        for id in ids {
            move_node(context, &bindings_context::MetadataQuery::Id(id), &target)?;
        }
        Ok(())
    }

    fn delete_by_ids(&mut self, context: Resource<MetadataContext>, ids: Vec<u64>) -> Result<()> {
        log::trace!("metadata context delete_by_ids {ids:?}");
        let context = self
            .resource_table
            .get_mut(&context)
            .inspect_err(|err| log::error!("{err}"))?;
        let mut seen = HashSet::new();
        for id in ids {
            if !seen.insert(id) {
                continue;
            }
            if context.find_with_parent(id).is_none() {
                // Already removed by an ancestor's cascade delete.
                continue;
            }
            delete_node(context, id)?;
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
