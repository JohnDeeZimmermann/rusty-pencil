//! The rectangle, diamond and ellipse tools: press, drag out a box, release to commit.

use pen_core::{Kind, WorldPt, WorldRect};

use crate::{Modifiers, ScreenPt, Tool};

/// A drag in progress with a shape tool. World points, so the camera can't move it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShapeDrag {
    tool: ShapeTool,
    /// Where the press landed.
    pub(crate) origin: WorldPt,
    /// Where the pointer is now.
    pub(crate) current: WorldPt,
    /// The pointer's last screen point, so a pan or zoom mid-drag keeps `current` under it.
    pub(crate) pointer: ScreenPt,
    pub(crate) mods: Modifiers,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapeTool {
    Rectangle,
    Diamond,
    Ellipse,
}

impl ShapeDrag {
    pub(crate) fn start(
        tool: Tool,
        origin: WorldPt,
        pointer: ScreenPt,
        mods: Modifiers,
    ) -> Option<Self> {
        let tool = match tool {
            Tool::Select | Tool::Arrow | Tool::Line | Tool::Freedraw | Tool::Hand => return None,
            Tool::Rectangle => ShapeTool::Rectangle,
            Tool::Diamond => ShapeTool::Diamond,
            Tool::Ellipse => ShapeTool::Ellipse,
        };
        Some(Self {
            tool,
            origin,
            current: origin,
            pointer,
            mods,
        })
    }

    /// The box the drag describes. Shift makes it square; Alt grows it from `origin` as the
    /// centre instead of the corner.
    pub(crate) fn bounds(&self) -> WorldRect {
        let (mut dx, mut dy) = (
            self.current.x - self.origin.x,
            self.current.y - self.origin.y,
        );
        if self.mods.shift {
            // The longer side wins, keeping the direction of each axis.
            let side = dx.abs().max(dy.abs());
            dx = side.copysign(dx);
            dy = side.copysign(dy);
        }
        if self.mods.alt {
            WorldRect::from_centre(self.origin, dx, dy)
        } else {
            let corner = WorldPt::new(self.origin.x + dx, self.origin.y + dy);
            WorldRect::from_corners(self.origin, corner)
        }
    }

    pub(crate) fn kind(&self) -> Kind {
        let bounds = self.bounds();
        match self.tool {
            ShapeTool::Rectangle => Kind::Rect(bounds),
            ShapeTool::Diamond => Kind::Diamond(bounds),
            ShapeTool::Ellipse => Kind::Ellipse(bounds),
        }
    }
}
