use sha2raw::Sha256;
use storage_proofs_core::{
    error::Result,
    hasher::Hasher,
    util::{data_at_node_offset, NODE_SIZE},
};

use super::{cache::ParentCache, graph::StackedBucketGraph};

pub fn create_label<H: Hasher>(
    graph: &StackedBucketGraph<H>,
    cache: Option<&mut ParentCache>,
    replica_id: &H::Domain,
    layer_labels: &mut [u8],
    layer_index: usize,
    node: usize,
) -> Result<()> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 32];

    buffer[..4].copy_from_slice(&(layer_index as u32).to_be_bytes());
    buffer[4..12].copy_from_slice(&(node as u64).to_be_bytes());
    hasher.input(&[AsRef::<[u8]>::as_ref(replica_id), &buffer[..]][..]);

    // hash parents for all non 0 nodes
    let hash = if node > 0 {
        // prefetch previous node, which is always a parent
        let prev = &layer_labels[(node - 1) * NODE_SIZE..node * NODE_SIZE];
        prefetch!(prev.as_ptr() as *const i8);

        graph.copy_parents_data(node as u32, &*layer_labels, hasher, cache)?
    } else {
        hasher.finish()
    };

    // store the newly generated key
    let start = data_at_node_offset(node);
    let end = start + NODE_SIZE;
    layer_labels[start..end].copy_from_slice(&hash[..]);

    // strip last two bits, to ensure result is in Fr.
    layer_labels[end - 1] &= 0b0011_1111;

    Ok(())
}

pub fn create_label_exp<H: Hasher>(
    graph: &StackedBucketGraph<H>,
    rx_cache: &bounded_spsc_queue::Consumer<[u32; 14]>,//cache: Option<&mut ParentCache>,
    replica_id: &H::Domain,
    rx_data: &bounded_spsc_queue::Consumer<[u8; 448]>,//exp_parents_data: &[u8],
    layer_labels: &mut [u8],
    layer_index: usize,
    node: usize,
) -> Result<()> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 32];

    buffer[0..4].copy_from_slice(&(layer_index as u32).to_be_bytes());
    buffer[4..12].copy_from_slice(&(node as u64).to_be_bytes());
    hasher.input(&[AsRef::<[u8]>::as_ref(replica_id), &buffer[..]][..]);

    // hash parents for all non 0 nodes
    let hash = if node > 0 {
        // prefetch previous node, which is always a parent
        let prev = &layer_labels[(node - 1) * NODE_SIZE..node * NODE_SIZE];
        prefetch!(prev.as_ptr() as *const i8);

        // graph.copy_parents_data_exp(node as u32, &*layer_labels, exp_parents_data, hasher, cache)?
        graph.my_copy_parents_data_exp(&*layer_labels, rx_data, hasher, rx_cache)?
    } else {
        hasher.finish()
    };

    // store the newly generated key
    let start = data_at_node_offset(node);
    let end = start + NODE_SIZE;
    layer_labels[start..end].copy_from_slice(&hash[..]);

    // strip last two bits, to ensure result is in Fr.
    layer_labels[end - 1] &= 0b0011_1111;

    Ok(())
}
