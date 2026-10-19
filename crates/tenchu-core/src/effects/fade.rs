use super::*;

#[derive(Clone, Debug)]
pub struct Fade {
    pub color: [u8; 3],
    pub priority: i32,
    pub start: i32,
    pub end: i32,
    pub mode: u8,
}

impl Effects {
    pub fn fade(&mut self, color: [u8; 3], priority: i32, clock: i32) {
        let slot = self.allocate();
        self.slots[slot] = Some(Effect::Fade(Fade {
            color,
            priority,
            start: clock,
            end: clock.wrapping_add(5),
            mode: 0,
        }));
    }

    pub(super) fn update_fade(&mut self, fade: &mut Fade, clock: i32) -> bool {
        let elapsed = clock.wrapping_sub(fade.start) as u32;
        let duration = fade.end.wrapping_sub(fade.start) as u32;
        let reached = clock as u32 >= fade.end as u32;
        let color = match fade.mode {
            0 => {
                let color = fade
                    .color
                    .map(|v| (elapsed.wrapping_mul(u32::from(v)) / duration) as u8);
                if reached {
                    fade.mode = 1;
                    fade.end = fade.end.wrapping_add(3);
                }
                color
            }
            1 => {
                if reached {
                    fade.mode = 2;
                    fade.end = fade.end.wrapping_add(40);
                }
                fade.color
            }
            _ => {
                if reached {
                    return false;
                }
                fade.color.map(|v| {
                    (duration.wrapping_sub(elapsed).wrapping_mul(u32::from(v)) / duration) as u8
                })
            }
        };
        self.fades.push((fade.priority, color));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle;
    use std::fmt::Write;

    #[test]
    #[ignore = "requires TENCHU_DECOMP; compares fade colors and expiry across clock rewinds and signed wrapping"]
    fn fade_lifetimes_match_original_c() {
        let source = oracle::source("EFFECT.C");
        let routines = ["void draw_fade_(", "void set_fade_("]
            .map(|s| oracle::function(&source, s).replace("long ", "int32_t "))
            .join("\n");
        let harness = format!(
            r#"
#include <stdint.h>
#include <stdio.h>
#include <assert.h>
typedef uint8_t u8;typedef uint32_t u32;
typedef struct {{u8 r,g,b;int32_t priority,start_time,end_time;u8 mode;}} FadeType;
typedef struct TEffectSlot {{void (*proc)(struct TEffectSlot *);union {{FadeType fade;}} param;}} TEffectSlot;
typedef struct {{struct {{int x0,y0,x1,y1,x2,y2,x3,y3;u8 r0,g0,b0;}} ply;}} POLY_XF4;
static TEffectSlot slot;static POLY_XF4 packet;static int32_t GameClock;static int drawn;
static struct {{int org[4096];}} table,*OTablePt=&table;
#define SCREEN_W 320
#define SCREEN_H 240
#define GPU_BLEND_ADD 1
#define FADE_MODE_IN 0
#define FADE_MODE_HOLD 1
#define FADE_MODE_OUT 2
#define SetPolyXF4(...) ((void)0)
static void *GsGetWorkBase(void){{return &packet;}}
static void GsSetWorkBase(void *p){{}}
static void AddXF4(void *p,POLY_XF4 *quad){{drawn=1;}}
static TEffectSlot *GetFreeEffectSlot(void){{return &slot;}}
{routines}
int main(void){{
 int a[6];while(scanf("%d",a)==1){{for(int i=1;i<6;i++)assert(scanf("%d",a+i)==1);
 GameClock=a[0];set_fade_(a[1],a[2],a[3],a[4]);
 for(int i=0;i<64;i++){{int delta;assert(scanf("%d",&delta)==1);GameClock+=delta;drawn=0;packet.ply.r0=packet.ply.g0=packet.ply.b0=0;
 if(slot.proc)slot.proc(&slot);
 printf("%d %d %d %d %d %d %d %d\n",slot.proc!=0,slot.param.fade.mode,slot.param.fade.start_time,slot.param.fade.end_time,drawn,packet.ply.r0,packet.ply.g0,packet.ply.b0);
 }}
 }}
}}
"#
        );
        let mut input = String::new();
        let mut expected = Vec::new();
        for case in 0..48 {
            let mut clock = [0i32, 90, i32::MAX - 8, i32::MIN + 5][case % 4];
            let color = [255, (case * 5) as u8, 127];
            let priority = (case % 3) as i32;
            writeln!(
                input,
                "{clock} {} {} {} {priority} 0",
                color[0], color[1], color[2]
            )
            .unwrap();
            let mut effects = Effects::default();
            let mut fade = Fade {
                color,
                priority,
                start: clock,
                end: clock.wrapping_add(5),
                mode: 0,
            };
            let mut alive = true;
            for i in 0..64 {
                let delta = if i == 12 && case % 3 == 0 { -30 } else { 1 };
                write!(input, "{delta} ").unwrap();
                clock = clock.wrapping_add(delta);
                effects.fades.clear();
                if alive {
                    alive = effects.update_fade(&mut fade, clock);
                }
                let color = effects.fades.first().map_or([0; 3], |(_, c)| *c);
                let mut state = vec![
                    i32::from(alive),
                    i32::from(fade.mode),
                    fade.start,
                    fade.end,
                    i32::from(!effects.fades.is_empty()),
                ];
                state.extend(color.map(i32::from));
                expected.push(state);
            }
            input.push('\n');
        }
        assert_eq!(expected, oracle::run(&harness, &input));
    }
}
