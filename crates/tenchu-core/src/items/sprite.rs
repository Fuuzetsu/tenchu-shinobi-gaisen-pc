use super::*;

pub(super) struct Sprite {
    pub graphic: effects::Graphic,
    pub scale: i32,
    pub color: u8,
    pub rotation: i32,
}

impl Items {
    pub(super) fn draw_sprite(
        &mut self,
        source: ModelId,
        template: ModelId,
        sprite: Sprite,
        ctx: &mut Context,
    ) -> Result<()> {
        ctx.scene.copy_coordinate(template, source);
        let matrix = ctx.camera.view.compose(&ctx.scene.world(template)?);
        let Some(clip) = visibility::clip(&matrix, ctx.scene.models[template].attribute, [0; 3])
        else {
            return Ok(());
        };
        let depth = clip.depth as i16 - 5;
        if depth < 1 {
            return Ok(());
        }
        let scale = ((sprite.scale >> 2).wrapping_mul(300) / i32::from(depth)) as i16;
        let drawing = effects::Drawing {
            graphic: sprite.graphic,
            xy: visibility::project(&matrix, [0; 3]).0,
            scale: [i32::from(scale); 2],
            color: [sprite.color; 3],
            rotation: sprite.rotation,
            depth,
        };
        ctx.drawing.bytes += drawing.bytes();
        self.drawings.push(drawing);
        Ok(())
    }
}
