use super::*;

#[derive(Clone, Debug)]
pub struct TextureScroll {
    pub offset: [i16; 2],
    pub velocity: [i16; 2],
    pub destination: [u16; 4],
    pub source: [u16; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct TextureMove {
    pub source: [u16; 4],
    pub destination: [u16; 2],
}

impl TextureScroll {
    pub fn step(&mut self) -> TextureMove {
        for axis in 0..2 {
            self.offset[axis] =
                ((i32::from(self.offset[axis]) + i32::from(self.velocity[axis])) as u32
                    % (u32::from(self.destination[axis + 2]) << 4)) as i16;
        }
        TextureMove {
            source: [
                self.source[0].wrapping_add((self.offset[0] / 16) as u16),
                self.source[1].wrapping_add((self.offset[1] / 16) as u16),
                self.destination[2],
                self.destination[3],
            ],
            destination: [self.destination[0], self.destination[1]],
        }
    }
}

impl Effects {
    pub fn texture_scroll(&mut self, destination: [u16; 4], source: [u16; 2], velocity: [i16; 2]) {
        let slot = self.allocate();
        self.slots[slot] = Some(Effect::TextureScroll(TextureScroll {
            offset: [0; 2],
            destination,
            source,
            velocity,
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle;
    use std::fmt::Write;

    #[test]
    #[ignore = "requires TENCHU_DECOMP; unsigned scrolling arithmetic, narrow accumulators and original copy rectangles"]
    fn scrolling_matches_original_c() {
        let source = oracle::source("EFFECT.C");
        let function = oracle::function(&source, "void UpdateTexScroll(TEffectSlot *ef)");
        let harness = [r#"
#include <stdint.h>
#include <stdio.h>
typedef uint32_t u32;typedef struct {int16_t x,y,w,h;} RECT;
typedef struct {int16_t px,py,vx,vy,x,y,sx,sy;RECT image;} TexScroll;
typedef struct {struct {TexScroll texscroll;}param;} TEffectSlot;
typedef struct {int words[6];} DR_MOVE;
static DR_MOVE primitive;static struct {int org;} ot,*OTablePt=&ot;
#define TEXSCROLL_SUBPIXEL_BITS 4
#define TEXSCROLL_SUBPIXEL_SCALE 16
static void*GsGetWorkBase(void){return &primitive;}
static void GsSetWorkBase(void*p){}
static void SetDrawMove(void*p,RECT*r,int x,int y){}
static void AddPrim(int p,void*q){}
"#,function,r#"
int main(void){int a[6];while(scanf("%d",a)==1){for(int i=1;i<6;i++)if(scanf("%d",a+i)!=1)return 2;
TEffectSlot e={0};TexScroll*t=&e.param.texscroll;*t=(TexScroll){a[0],a[1],a[2],a[3],704,320,832,256,{704,320,a[4],a[5]}};
for(int frame=0;frame<80;frame++){UpdateTexScroll(&e);printf("%d %d %d %d %d %d %d %d\n",t->px,t->py,t->image.x,t->image.y,t->image.w,t->image.h,t->x,t->y);}
}return 0;}
"#].concat();
        let mut input = String::new();
        let mut expected = Vec::new();
        for case in 0..512 {
            let offset = [[-32768, 32767], [-1, 511], [500, 300], [0, 0]][case % 4];
            let velocity = [
                [-32768, 32767],
                [-48, -10],
                [16, 0],
                [0, 2],
                [1, -1],
                [1000, -256],
            ][case / 4 % 6];
            let size = [[32, 32], [31, 17], [100, 500], [1, 1]][case / 24 % 4];
            writeln!(
                input,
                "{} {} {} {} {} {}",
                offset[0], offset[1], velocity[0], velocity[1], size[0], size[1]
            )
            .unwrap();
            let mut scroll = TextureScroll {
                offset,
                velocity,
                destination: [704, 320, size[0], size[1]],
                source: [832, 256],
            };
            for _ in 0..80 {
                let step = scroll.step();
                let mut row = scroll.offset.map(i32::from).to_vec();
                row.extend(step.source.map(i32::from));
                row.extend(step.destination.map(i32::from));
                expected.push(row);
            }
        }
        let actual = oracle::run(&harness, &input);
        assert_eq!(actual, expected);
    }

    #[test]
    fn layout_reset_retains_scroll_slots_and_the_allocation_cursor() {
        let mut effects = Effects::default();
        effects.texture_scroll([704, 320, 32, 32], [832, 256], [16, 0]);
        effects.flash(None, Position::ZERO, 4096, 4);
        effects.texture_scroll([736, 256, 32, 32], [832, 320], [0, 2]);
        let cursor = effects.cursor;
        effects.reset();
        assert_eq!(effects.cursor, cursor);
        assert_eq!(effects.slots.iter().flatten().count(), 2);
        assert!(matches!(effects.slots[1], Some(Effect::TextureScroll(_))));
        assert!(matches!(effects.slots[5], Some(Effect::TextureScroll(_))));
        effects.flash(None, Position::ZERO, 4096, 4);
        assert!(matches!(effects.slots[7], Some(Effect::Flash(_))));
    }
}
