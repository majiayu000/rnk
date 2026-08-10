//! Target-exact snapshot adapter over a validated layout candidate.

use crate::{
    core::{Display, Element, ElementType, Overflow},
    layout::{
        Axis, AxisClip, CellPoint, CellSpan, CellVector, GeometryField, LayoutSnapshotBuilder,
        LayoutSnapshotError, PreparedSnapshotFrame, SnapshotBuildReport, SnapshotBuildStrategy,
        SnapshotIdentity, SnapshotNodeIndex, TextFlowSemanticStamp, checked_add, checked_extent,
        checked_finite, checked_subtract, quantize_rect,
    },
};
use std::sync::Arc;

use super::{LayoutEngine, incremental::ElementVNodeSnapshot};
use crate::reconciler::{ScopedIdentityArena, ScopedNodeIdentity};

impl LayoutEngine {
    pub(crate) fn element_id_for_snapshot_identity(
        target: &Element,
        identity: &SnapshotIdentity,
    ) -> Option<crate::core::ElementId> {
        let mut arena = ScopedIdentityArena::default();
        let snapshot = ElementVNodeSnapshot::from_element(target, &mut arena).ok()?;
        snapshot
            .element_scopes
            .into_iter()
            .find_map(|(element_id, scoped)| (scoped == *identity.scoped()).then_some(element_id))
    }

    /// Build a read-only terminal-cell snapshot from the current Element frame.
    ///
    /// This compatibility producer does not mutate the engine. Incremental
    /// frame preparation uses the same adapter before publishing its candidate.
    ///
    /// # Errors
    ///
    /// Returns a closed [`LayoutSnapshotError`] when target identity, layout,
    /// TextFlow, or signed cell geometry is invalid.
    pub fn try_snapshot(
        &self,
        target: &Element,
    ) -> Result<(PreparedSnapshotFrame, SnapshotBuildReport), LayoutSnapshotError> {
        self.try_build_snapshot_for(
            target,
            self.last_width,
            self.last_height,
            SnapshotBuildStrategy::InitialFull,
            0,
            0,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_build_snapshot_for(
        &self,
        target: &Element,
        width: u16,
        height: u16,
        strategy: SnapshotBuildStrategy,
        patch_count: usize,
        rebuild_count: usize,
    ) -> Result<(PreparedSnapshotFrame, SnapshotBuildReport), LayoutSnapshotError> {
        let mut builder = LayoutSnapshotBuilder::new(width, height);
        let derived_identities;
        let identities = if self.element_scopes.is_empty() {
            let mut arena = ScopedIdentityArena::default();
            derived_identities = ElementVNodeSnapshot::from_element(target, &mut arena)
                .map_err(|_| LayoutSnapshotError::MissingIdentity {
                    element_id: target.id,
                })?
                .element_scopes;
            &derived_identities
        } else {
            &self.element_scopes
        };
        let viewport_clip = AxisClip::from_rect(builder.viewport());
        let root = self
            .snapshot_subtree(
                target,
                None,
                0.0,
                0.0,
                viewport_clip,
                identities,
                &mut builder,
            )?
            .ok_or(LayoutSnapshotError::MissingIdentity {
                element_id: target.id,
            })?;
        Ok(builder.finish(root, strategy, patch_count, rebuild_count))
    }

    #[allow(clippy::too_many_arguments)]
    fn snapshot_subtree(
        &self,
        element: &Element,
        parent: Option<SnapshotNodeIndex>,
        parent_child_x: f64,
        parent_child_y: f64,
        inherited_clip: AxisClip,
        identities: &std::collections::HashMap<crate::core::ElementId, ScopedNodeIdentity>,
        builder: &mut LayoutSnapshotBuilder,
    ) -> Result<Option<SnapshotNodeIndex>, LayoutSnapshotError> {
        if element.style.display == Display::None
            || element.element_type == ElementType::VirtualText
        {
            return Ok(None);
        }

        let scoped =
            identities
                .get(&element.id)
                .cloned()
                .ok_or(LayoutSnapshotError::MissingIdentity {
                    element_id: element.id,
                })?;
        let identity = SnapshotIdentity::from_scoped(scoped);
        let node_id = self.node_map.get(&element.id).copied().ok_or_else(|| {
            LayoutSnapshotError::MissingLayout {
                identity: identity.clone(),
            }
        })?;
        if self.taffy.get_node_context(node_id).is_none() {
            return Err(LayoutSnapshotError::MissingLayout { identity });
        }
        let layout = self.taffy.unrounded_layout(node_id);

        let local_x = checked_finite(&identity, GeometryField::X, layout.location.x)?;
        let local_y = checked_finite(&identity, GeometryField::Y, layout.location.y)?;
        let width = checked_extent(&identity, Axis::X, GeometryField::Width, layout.size.width)?;
        let height = checked_extent(
            &identity,
            Axis::Y,
            GeometryField::Height,
            layout.size.height,
        )?;
        let absolute_left = checked_add(&identity, parent_child_x, local_x)?;
        let absolute_top = checked_add(&identity, parent_child_y, local_y)?;
        let absolute_right = checked_add(&identity, absolute_left, width)?;
        let absolute_bottom = checked_add(&identity, absolute_top, height)?;
        let border_bounds = quantize_rect(
            &identity,
            absolute_left,
            absolute_top,
            absolute_right,
            absolute_bottom,
        )?;

        let left_inset = checked_add(
            &identity,
            checked_finite(&identity, GeometryField::LeftInset, layout.border.left)?,
            checked_finite(
                &identity,
                GeometryField::LeftInset,
                element.style.padding.left,
            )?,
        )?;
        let top_inset = checked_add(
            &identity,
            checked_finite(&identity, GeometryField::TopInset, layout.border.top)?,
            checked_finite(
                &identity,
                GeometryField::TopInset,
                element.style.padding.top,
            )?,
        )?;
        let right_inset = checked_add(
            &identity,
            checked_finite(&identity, GeometryField::RightInset, layout.border.right)?,
            checked_finite(
                &identity,
                GeometryField::RightInset,
                element.style.padding.right,
            )?,
        )?;
        let bottom_inset = checked_add(
            &identity,
            checked_finite(&identity, GeometryField::BottomInset, layout.border.bottom)?,
            checked_finite(
                &identity,
                GeometryField::BottomInset,
                element.style.padding.bottom,
            )?,
        )?;
        let content_left = checked_add(&identity, absolute_left, left_inset)?;
        let content_top = checked_add(&identity, absolute_top, top_inset)?;
        let content_right = checked_subtract(&identity, absolute_right, right_inset)?;
        let content_bottom = checked_subtract(&identity, absolute_bottom, bottom_inset)?;
        let attempted_content = quantize_rect(
            &identity,
            content_left,
            content_top,
            content_right,
            content_bottom,
        )?;
        if content_right < content_left || content_bottom < content_top {
            return Err(LayoutSnapshotError::ReversedContentBounds {
                identity,
                border_bounds,
                attempted_content_bounds: attempted_content,
            });
        }
        let content_bounds = attempted_content.intersect(border_bounds);
        let text_origin = CellPoint::checked(attempted_content.left(), attempted_content.top());

        let mut effective_clip = inherited_clip;
        if matches!(
            element.style.overflow_x,
            Overflow::Hidden | Overflow::Scroll
        ) {
            effective_clip = AxisClip::checked(
                effective_clip.x().intersect(CellSpan::checked(
                    content_bounds.left(),
                    content_bounds.right(),
                )),
                effective_clip.y(),
            );
        }
        if matches!(
            element.style.overflow_y,
            Overflow::Hidden | Overflow::Scroll
        ) {
            effective_clip = AxisClip::checked(
                effective_clip.x(),
                effective_clip.y().intersect(CellSpan::checked(
                    content_bounds.top(),
                    content_bounds.bottom(),
                )),
            );
        }

        let scroll_x = i32::from(element.scroll_offset_x.unwrap_or(0));
        let scroll_y = i32::from(element.scroll_offset_y.unwrap_or(0));
        let scroll_transform = CellVector::checked(-scroll_x, -scroll_y);
        let text_flow = if element.element_type == ElementType::Text {
            let current = self.current_text_flow(element.id).ok_or_else(|| {
                LayoutSnapshotError::MissingTextFlowRevision {
                    identity: identity.clone(),
                }
            })?;
            let cell_width =
                usize::try_from(attempted_content.width().max(0)).unwrap_or(usize::MAX);
            let flow = if current.cache_identity().options.max_width == cell_width {
                current
            } else {
                let mut options = current.cache_identity().options.clone();
                options.max_width = cell_width;
                Arc::new(
                    crate::layout::TextFlow::try_build(&current.cache_identity().input, &options)
                        .map_err(|source| LayoutSnapshotError::TextFlowRevision {
                        identity: identity.clone(),
                        source,
                    })?,
                )
            };
            Some(TextFlowSemanticStamp::checked(flow))
        } else {
            None
        };

        let index = builder.push(
            element.id,
            identity.clone(),
            parent,
            border_bounds,
            content_bounds,
            text_origin,
            effective_clip,
            scroll_transform,
            text_flow,
        )?;

        let child_origin_x = checked_subtract(&identity, absolute_left, f64::from(scroll_x))?;
        let child_origin_y = checked_subtract(&identity, absolute_top, f64::from(scroll_y))?;
        let mut children = Vec::new();
        for child in &element.children {
            if let Some(child_index) = self.snapshot_subtree(
                child,
                Some(index),
                child_origin_x,
                child_origin_y,
                effective_clip,
                identities,
                builder,
            )? {
                children.push(child_index);
            }
        }
        builder.set_children(index, children);
        Ok(Some(index))
    }
}
