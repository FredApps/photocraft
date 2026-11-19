use crate::{Document, Layer, LayerId, LayerPath};
use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

/// Root layer storage exposed through the usual `document.layers` field access.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayerFields {
    /// Bottom to top, as for group children.
    pub layers: Vec<Layer>,
}

/// The tree owns its index. Replacing the tree also replaces its cache.
#[derive(Default)]
pub struct LayerTree {
    fields: LayerFields,
    index: OnceLock<LayerIndex>,
    #[cfg(test)]
    builds: std::sync::atomic::AtomicUsize,
}

impl From<Vec<Layer>> for LayerTree {
    fn from(layers: Vec<Layer>) -> Self {
        Self { fields: LayerFields { layers }, ..Self::default() }
    }
}

impl Clone for LayerTree {
    fn clone(&self) -> Self {
        self.fields.layers.clone().into()
    }
}

impl PartialEq for LayerTree {
    fn eq(&self, other: &Self) -> bool {
        self.fields == other.fields
    }
}

impl std::fmt::Debug for LayerTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fields.layers.fmt(f)
    }
}

impl Deref for Document {
    type Target = LayerFields;

    fn deref(&self) -> &LayerFields {
        &self.layer_tree.fields
    }
}

impl DerefMut for Document {
    fn deref_mut(&mut self) -> &mut LayerFields {
        self.layer_tree.index.take();
        &mut self.layer_tree.fields
    }
}

// One parent link per layer keeps storage linear even for nested groups.
struct Entry {
    parent: Option<usize>,
    slot: usize,
}

struct LayerIndex {
    entries: Vec<Entry>,
    first: HashMap<LayerId, usize>,
    repeated: HashMap<LayerId, Vec<usize>>,
}

impl LayerIndex {
    fn new(layers: &[Layer]) -> Self {
        let mut index = Self { entries: Vec::new(), first: HashMap::new(), repeated: HashMap::new() };
        let mut stack = vec![(None, layers.iter().enumerate())];
        while let Some((parent, iter)) = stack.last_mut() {
            let Some((slot, layer)) = iter.next() else {
                stack.pop();
                continue;
            };
            let ordinal = index.entries.len();
            index.entries.push(Entry { parent: *parent, slot });
            let first = *index.first.entry(layer.id).or_insert(ordinal);
            if first != ordinal {
                index.repeated.entry(layer.id).or_default().push(ordinal);
            }
            if let Some(children) = layer.children() {
                stack.push((Some(ordinal), children.iter().enumerate()));
            }
        }
        index
    }

    fn path<'a>(&'a self, mut entry: &'a Entry) -> LayerPath {
        let mut path = vec![entry.slot];
        while let Some(parent) = entry.parent {
            entry = &self.entries[parent];
            path.push(entry.slot);
        }
        path.reverse();
        path
    }
}

impl LayerTree {
    fn index(&self) -> &LayerIndex {
        self.index.get_or_init(|| {
            #[cfg(test)]
            self.builds.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            LayerIndex::new(&self.fields.layers)
        })
    }

    pub(crate) fn path_of(&self, id: LayerId) -> Option<LayerPath> {
        let index = self.index();
        let entry = &index.entries[*index.first.get(&id)?];
        Some(index.path(entry))
    }

    pub(crate) fn find(&self, id: LayerId) -> Option<&Layer> {
        let index = self.index();
        let entry = &index.entries[*index.first.get(&id)?];
        let layers = &self.fields.layers;
        if entry.parent.is_none() {
            return layers.get(entry.slot);
        }
        let mut siblings = layers.as_slice();
        let path = index.path(entry);
        let (slot, parents) = path.split_last()?;
        for &parent in parents {
            siblings = siblings.get(parent)?.children()?;
        }
        siblings.get(*slot)
    }
}

impl Document {
    /// Requested IDs in bottom-to-top document order. A canonical selection borrows
    /// its existing slice. Missing IDs and repeated requests have no effect.
    pub fn ordered_layer_ids<'a>(&self, ids: &'a [LayerId]) -> Cow<'a, [LayerId]> {
        if ids.is_empty() {
            return Cow::Borrowed(ids);
        }
        let index = self.layer_tree.index();
        let mut previous = None;
        let in_order = ids.iter().all(|id| {
            let Some(&ordinal) = index.first.get(id) else { return false };
            let ordered = previous.is_none_or(|last| last < ordinal) && !index.repeated.contains_key(id);
            previous = Some(ordinal);
            ordered
        });
        if in_order {
            return Cow::Borrowed(ids);
        }

        let mut ordered = Vec::new();
        for &id in ids {
            if let Some(&ordinal) = index.first.get(&id) {
                ordered.push((ordinal, id));
                if let Some(more) = index.repeated.get(&id) {
                    ordered.extend(more.iter().map(|&slot| (slot, id)));
                }
            }
        }
        ordered.sort_unstable_by_key(|&(slot, _)| slot);
        ordered.dedup_by_key(|(slot, _)| *slot);
        Cow::Owned(ordered.into_iter().map(|(_, id)| id).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorMode, SampleType, Size};
    use std::sync::atomic::Ordering;

    #[test]
    fn repeated_reads_share_the_index_until_a_layer_edit() {
        let mut doc = Document::new("Index", Size::new(8, 8), ColorMode::Rgb, SampleType::U8);
        let format = doc.pixel_format();
        doc.layers = (0..200).map(|i| Layer::raster(i.to_string(), format)).collect();
        let first = doc.layers[0].id;
        let last = doc.layers[199].id;
        let selection = [first, last];
        for _ in 0..100 {
            assert_eq!(doc.layer(last).map(|layer| layer.id), Some(last));
            assert!(matches!(doc.ordered_layer_ids(&selection), Cow::Borrowed(_)));
        }
        assert_eq!(doc.layer_tree.builds.load(Ordering::Relaxed), 1);
        let original = doc.clone();
        doc.layers.reverse();
        assert_eq!(&*doc.ordered_layer_ids(&selection), &[last, first]);
        assert_eq!(doc.path_of(last), Some(vec![0]));
        assert_eq!(original.path_of(last), Some(vec![199]));
        assert_eq!(doc.layer_tree.builds.load(Ordering::Relaxed), 2);
    }
}
