use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HitTestBehavior {
    #[default]
    DeferToChild,
    Opaque,
    Translucent,
}
/// A translation-only layout hit. Entries are ordered deepest/frontmost first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitTestEntry {
    pub id: Id,
    pub local_position: Pos2,
    /// Origin of this box relative to the root used for the hit test.
    pub offset: Vec2,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HitTestResult {
    pub path: Vec<HitTestEntry>,
    /// True when this subtree blocks a sibling/root behind it.
    pub absorbed: bool,
}
impl LayoutBox {
    /// Hit the current geometry without painting. Ancestor bounds gate hits,
    /// even when visual overflow is unclipped. Translucent entries can precede
    /// a blocking sibling; an opaque descendant still blocks siblings behind it.
    pub fn hit_test_path(&self, point: Pos2) -> Vec<HitTestEntry> {
        self.hit_test_result(point).path
    }
    pub fn hit_test_result(&self, point: Pos2) -> HitTestResult {
        let mut path = Vec::new();
        let absorbed = self.hit_path(point, Vec2::ZERO, &mut path);
        HitTestResult { path, absorbed }
    }
    fn hit_path(&self, point: Pos2, offset: Vec2, result: &mut Vec<HitTestEntry>) -> bool {
        if self.offstage {
            return false;
        }
        if !(point.x >= 0.0 && point.y >= 0.0 && point.x < self.size.x && point.y < self.size.y) {
            return false;
        }
        let child_hit = self.children.iter().rev().any(|child| {
            child
                .layout
                .hit_path(point - child.offset, offset + child.offset, result)
        });
        let behavior = self.hit_behavior.unwrap_or(if self.hit_self {
            HitTestBehavior::Opaque
        } else {
            HitTestBehavior::DeferToChild
        });
        let hit = child_hit || behavior == HitTestBehavior::Opaque;
        if hit || behavior == HitTestBehavior::Translucent {
            result.push(HitTestEntry {
                id: self.id,
                local_position: point,
                offset,
            });
        }
        hit
    }
}
