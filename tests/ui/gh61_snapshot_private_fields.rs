use rnk::layout::{CellRect, SnapshotIdentity};

fn main() {
    let _forged_rect = CellRect {
        left: 0,
        top: 0,
        right: 1,
        bottom: 1,
    };
    let _forged_identity = SnapshotIdentity::from_scoped;
}
