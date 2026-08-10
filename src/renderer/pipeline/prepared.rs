//! Delayed dynamic-frame preparation and publication.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::core::{Display, Element, ElementType, VNode};
use crate::layout::{
    BoundPreparedLayoutFrame, CheckedIncrementalLayoutReport, CheckedLayoutSnapshot,
    FullRebuildError, IncrementalLayoutError, LayoutEngine, LayoutSnapshotError,
    LegacyLayoutSnapshotError, PreparedLayoutCommitError, PreparedLayoutFrame,
    PreparedSnapshotFrame, RebuildFailure, TransactionalLayoutError,
};
use crate::reconciler::{ScopedNodeIdentity, SiblingIdentity};
use crate::renderer::{
    CheckedRenderError, DynamicFrameError, LayoutRenderError, Output, RecoveredSnapshotRenderError,
    SnapshotRenderError, TextCoordinateError, TextRenderError, TransactionalFrameError,
    try_render_element_snapshot_checked,
};
use crate::runtime::RuntimeContext;

use super::RenderPipeline;

pub(crate) struct PreparedDynamicFrame {
    layout: PreparedLayoutFrame,
    rendered: String,
    measurements: CheckedLayoutSnapshot,
    raw_node_candidates: HashMap<SiblingIdentity, Vec<ScopedNodeIdentity>>,
    key_aliases: HashMap<String, Vec<(ScopedNodeIdentity, SiblingIdentity)>>,
}

pub(crate) struct BoundPreparedDynamicFrame<'a> {
    layout: BoundPreparedLayoutFrame<'a>,
    rendered: String,
    measurements: CheckedLayoutSnapshot,
    raw_node_candidates: HashMap<SiblingIdentity, Vec<ScopedNodeIdentity>>,
    key_aliases: HashMap<String, Vec<(ScopedNodeIdentity, SiblingIdentity)>>,
}

impl PreparedDynamicFrame {
    pub(crate) fn rendered(&self) -> &str {
        &self.rendered
    }

    #[cfg(test)]
    pub(crate) fn layout(&self) -> &PreparedLayoutFrame {
        &self.layout
    }

    pub(crate) fn commit(
        self,
        layout_engine: &mut LayoutEngine,
        runtime_context: &Rc<RefCell<RuntimeContext>>,
        previous_vnode: &mut Option<VNode>,
    ) -> String {
        self.commit_with_runtime(
            layout_engine,
            &mut runtime_context.borrow_mut(),
            previous_vnode,
        )
    }

    pub(crate) fn commit_with_runtime(
        self,
        layout_engine: &mut LayoutEngine,
        runtime_context: &mut RuntimeContext,
        previous_vnode: &mut Option<VNode>,
    ) -> String {
        self.bind(layout_engine)
            .unwrap_or_else(|error| panic!("dynamic frame publication failed: {error}"))
            .commit_with_runtime(runtime_context, previous_vnode)
    }

    pub(crate) fn bind(
        self,
        layout_engine: &mut LayoutEngine,
    ) -> Result<BoundPreparedDynamicFrame<'_>, PreparedLayoutCommitError> {
        Ok(BoundPreparedDynamicFrame {
            layout: self.layout.bind(layout_engine)?,
            rendered: self.rendered,
            measurements: self.measurements,
            raw_node_candidates: self.raw_node_candidates,
            key_aliases: self.key_aliases,
        })
    }
}

impl BoundPreparedDynamicFrame<'_> {
    pub(crate) fn commit_with_runtime(
        self,
        runtime_context: &mut RuntimeContext,
        previous_vnode: &mut Option<VNode>,
    ) -> String {
        let (current_vnode, _) = self.layout.commit();
        runtime_context.set_measure_layouts_with_scoped_keys(
            self.measurements.element,
            self.measurements.scoped_vnode,
            self.measurements.vnode,
            self.raw_node_candidates,
            self.key_aliases,
        );
        *previous_vnode = Some(current_vnode);
        self.rendered
    }
}

impl RenderPipeline {
    pub(crate) fn prepare_dynamic_frame(
        dynamic_root: &Element,
        width: u16,
        height: u16,
        layout_engine: &LayoutEngine,
        previous_vnode: Option<&VNode>,
    ) -> Result<PreparedDynamicFrame, TransactionalFrameError> {
        Self::prepare_dynamic_frame_with_renderer(
            dynamic_root,
            width,
            height,
            layout_engine,
            previous_vnode,
            try_render_element_snapshot_checked,
        )
    }

    pub(super) fn prepare_dynamic_frame_with_renderer(
        dynamic_root: &Element,
        width: u16,
        height: u16,
        layout_engine: &LayoutEngine,
        previous_vnode: Option<&VNode>,
        renderer: impl FnOnce(
            &Element,
            &PreparedSnapshotFrame,
            &mut Output,
            f32,
            f32,
        ) -> Result<(), CheckedRenderError>,
    ) -> Result<PreparedDynamicFrame, TransactionalFrameError> {
        let layout = layout_engine
            .prepare_element_incremental(dynamic_root, previous_vnode, width, height)
            .map_err(TransactionalFrameError::Transaction)?;
        let candidate = layout.engine();
        let measurements = candidate
            .try_get_snapshot_measurements(layout.prepared_snapshot())
            .map_err(snapshot_error)?;
        let raw_node_candidates = candidate.raw_vnode_identity_candidates();
        let mut key_aliases = HashMap::new();
        collect_key_aliases(dynamic_root, candidate, &mut key_aliases, true)?;

        if dynamic_root.style.display == Display::None
            || dynamic_root.element_type == ElementType::VirtualText
        {
            return Ok(PreparedDynamicFrame {
                layout,
                rendered: String::new(),
                measurements,
                raw_node_candidates,
                key_aliases,
            });
        }
        let root_bounds = layout.snapshot().root().border_bounds();
        let content_width = u16::try_from(root_bounds.width().max(1))
            .unwrap_or(width)
            .min(width);
        let render_height = u16::try_from(root_bounds.height().max(1))
            .unwrap_or(height)
            .min(height);
        let mut output = Output::new(content_width, render_height);
        let recovered_incremental = match layout.report() {
            CheckedIncrementalLayoutReport::RecoveredFullRebuild {
                incremental_failure,
                ..
            } => Some(incremental_failure.clone()),
            _ => None,
        };
        if let Err(error) = renderer(
            dynamic_root,
            layout.prepared_snapshot(),
            &mut output,
            0.0,
            0.0,
        ) {
            let error = match (recovered_incremental, error) {
                (Some(incremental), CheckedRenderError::Snapshot(render)) => {
                    CheckedRenderError::RecoveredSnapshot(RecoveredSnapshotRenderError::new(
                        incremental,
                        render,
                    ))
                }
                (_, error) => error,
            };
            return Err(TransactionalFrameError::Render(error));
        }

        Ok(PreparedDynamicFrame {
            layout,
            rendered: output.render(),
            measurements,
            raw_node_candidates,
            key_aliases,
        })
    }
}

fn collect_key_aliases(
    element: &Element,
    layout_engine: &LayoutEngine,
    out: &mut HashMap<String, Vec<(ScopedNodeIdentity, SiblingIdentity)>>,
    is_root: bool,
) -> Result<(), TransactionalFrameError> {
    if element.style.display == Display::None || element.element_type == ElementType::VirtualText {
        return Ok(());
    }
    if let Some(key) = &element.key {
        let (identity, projection) = layout_engine
            .scoped_projection_for_element(element.id)
            .ok_or({
                TransactionalFrameError::Render(CheckedRenderError::Layout(if is_root {
                    LayoutRenderError::MissingRootLayout {
                        element_id: element.id,
                    }
                } else {
                    LayoutRenderError::MissingElementLayout {
                        element_id: element.id,
                    }
                }))
            })?;
        out.entry(key.clone())
            .or_default()
            .push((identity, projection));
    }
    for child in &element.children {
        collect_key_aliases(child, layout_engine, out, false)?;
    }
    Ok(())
}

fn snapshot_error(source: LegacyLayoutSnapshotError) -> TransactionalFrameError {
    let source = match source {
        LegacyLayoutSnapshotError::Lookup(source) => LayoutRenderError::LayoutLookup(source),
        LegacyLayoutSnapshotError::Invariant(source) => LayoutRenderError::Invariant(source),
    };
    TransactionalFrameError::Render(CheckedRenderError::Layout(source))
}

pub(super) fn legacy_dynamic_error(
    source: TransactionalFrameError,
    root_element_id: crate::core::ElementId,
) -> DynamicFrameError {
    match source {
        TransactionalFrameError::Upstream(source) => source,
        TransactionalFrameError::Transaction(TransactionalLayoutError::Upstream(source)) => {
            DynamicFrameError::Incremental(source)
        }
        TransactionalFrameError::Transaction(TransactionalLayoutError::InitialBuild(
            FullRebuildError {
                source: RebuildFailure::TextFlow(source),
                ..
            },
        )) => DynamicFrameError::Incremental(IncrementalLayoutError::TextFlow(source)),
        TransactionalFrameError::Render(CheckedRenderError::Text(source)) => {
            DynamicFrameError::Text(source)
        }
        TransactionalFrameError::Render(CheckedRenderError::Snapshot(
            SnapshotRenderError::Text { source, .. },
        )) => DynamicFrameError::Text(source),
        TransactionalFrameError::Render(CheckedRenderError::Layout(
            LayoutRenderError::LayoutLookup(source),
        )) => DynamicFrameError::LegacyLookup(source),
        TransactionalFrameError::Transaction(TransactionalLayoutError::Snapshot(
            LayoutSnapshotError::NonFiniteGeometry { .. },
        )) => DynamicFrameError::Text(TextRenderError::coordinate(
            root_element_id,
            TextCoordinateError::NonFinite,
        )),
        TransactionalFrameError::Transaction(TransactionalLayoutError::SnapshotBuild(source))
            if matches!(
                source.source_error(),
                LayoutSnapshotError::NonFiniteGeometry { .. }
            ) =>
        {
            DynamicFrameError::Text(TextRenderError::coordinate(
                root_element_id,
                TextCoordinateError::NonFinite,
            ))
        }
        TransactionalFrameError::Transaction(TransactionalLayoutError::RecoveredSnapshot(
            source,
        )) if matches!(
            source.snapshot_failure(),
            LayoutSnapshotError::NonFiniteGeometry { .. }
        ) =>
        {
            DynamicFrameError::Text(TextRenderError::coordinate(
                root_element_id,
                TextCoordinateError::NonFinite,
            ))
        }
        other => panic!("legacy dynamic frame cannot represent generalized error: {other}"),
    }
}

#[cfg(test)]
pub(super) mod tests;
