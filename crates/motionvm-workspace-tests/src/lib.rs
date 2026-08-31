//! The workspace's own rig. There is no library here: the tests under
//! `tests/` read the tree itself — manifests, sources, the member list — and
//! hold the rules no single crate can hold for itself, the layer law above
//! all. The crate depends on nothing, so it can sit outside both layers and
//! judge them.
