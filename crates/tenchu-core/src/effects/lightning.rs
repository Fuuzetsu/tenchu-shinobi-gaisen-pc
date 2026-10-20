use super::*;
use crate::math::Math;

impl Effects {
    pub fn lightning(
        &mut self,
        start: [i32; 3],
        end: [i32; 3],
        color: [u8; 3],
        math: &Math,
        view: View,
        random: &mut Random,
    ) -> u32 {
        let count =
            math.vector_distance(std::array::from_fn(|i| start[i].wrapping_sub(end[i]))) / 200;
        let mut previous = view.project(start);
        let mut bytes = 0;
        for step in 1..count {
            let point = std::array::from_fn(|i| {
                (end[i].wrapping_sub(start[i]).wrapping_mul(step) / count)
                    .wrapping_add(start[i])
                    .wrapping_add(random.next15() % 160 - 80)
            });
            // The branch roll recurses with generation zero, which emits nothing.
            random.next15();
            let current = view.project(point);
            if previous.1 > 0 && current.1 > 0 {
                self.lines.push(Line {
                    points: [previous.0, current.0],
                    depth: (current.1 >> 2).clamp(0, 1249),
                    color,
                });
                bytes += 20;
            }
            previous = current;
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle;
    use std::fmt::Write;

    #[test]
    #[ignore = "requires TENCHU_DISC and TENCHU_DECOMP; source lightning segment positions/RNG, projected through the separately checked SDK math"]
    fn lightning_segments_match_original_c_in_visible_and_hidden_views() {
        let source = oracle::source("EFFECT.C");
        let function = oracle::function(&source, "static void SetLightningI(VECTOR *start, VECTOR *end, int gen, short r, short g, short b)")
            .replace("long ", "int32_t ");
        let harness = [r#"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <assert.h>
typedef uint16_t u16;
typedef struct {int32_t vx,vy,vz,pad;} VECTOR;
typedef struct {int16_t vx,vy,vz,pad;} SVECTOR;
typedef struct {int attribute;uint8_t r,g,b;int16_t x0,y0,x1,y1;} GsLINE;
enum {FIXED_ONE=4096,GS_ATTR_SEMITRANS_ADD=1};
#define CLAMP_SORT_DEPTH(p,z) ((p)=(z)<0?0:(z)>1249?1249:(z))
static void *OTablePt;
static uint32_t seed;static int calls,root,n,points[2048][3];
static int game_rand(void) {seed=seed*0x41c64e6d+12345;calls++;return(seed>>16)&32767;}
#define rand game_rand
static int SquareRoot0(int x) {return root;}
static void PrepareLightningScreenPosition(void) {}
static void GetLightningScreenPosition(int x,int y,int z,SVECTOR*s) {
assert(n<2048);points[n][0]=x;points[n][1]=y;points[n][2]=z;n++;*s=(SVECTOR){0,0,1,0};}
static void GsSortLine(GsLINE*l,void*t,uint16_t z) {}
"#, &function, r#"
int main(void) {VECTOR start,end;int initial;while(scanf("%d %d %d %d %d %d %d %d",&start.vx,&start.vy,&start.vz,&end.vx,&end.vy,&end.vz,&root,&initial)==8) {
seed=(uint32_t)initial;calls=n=0;SetLightningI(&start,&end,1,100,100,200);
printf("%d %d %d",(int32_t)seed,calls,n);for(int i=0;i<n;i++)for(int j=0;j<3;j++)printf(" %d",points[i][j]);puts("");}return 0;}
"#].concat();
        let game = oracle::game(0, 0);
        let mut input = String::new();
        let mut cases = Vec::new();
        for case in 0..96 {
            let start: [i32; 3] =
                [[0, 0, 1000], [65500, -64000, -20000], [-300, 0, -2000]][case / 8 % 3];
            let span = [0, 199, 200, 399, 400, 4096, 20000, 65535][case % 8];
            let end = [
                start[0].wrapping_add(span),
                start[1],
                start[2].wrapping_add(span / 2),
            ];
            let delta: [i32; 3] = std::array::from_fn(|i| start[i].wrapping_sub(end[i]));
            let root = game.math.length(if delta.iter().any(|v| v.abs() > 4096) {
                delta.map(|v| v / 256)
            } else {
                delta
            });
            let seed = case as u32 * 871 + 7;
            writeln!(
                input,
                "{} {} {} {} {} {} {root} {seed}",
                start[0], start[1], start[2], end[0], end[1], end[2]
            )
            .unwrap();
            cases.push((start, end, seed));
        }
        let original = oracle::run(&harness, &input);
        for (case, ((start, end, seed), result)) in cases.into_iter().zip(original).enumerate() {
            let points: Vec<[i32; 3]> = result[3..]
                .chunks_exact(3)
                .map(|p| p.try_into().unwrap())
                .collect();
            assert_eq!(points.len(), result[2] as usize);
            for yaw in [0, 2048] {
                let view = View {
                    position: [0; 3],
                    matrix: Matrix {
                        rotation: game.math.rotate_yxz([0, yaw, 0]),
                        ..Matrix::default()
                    },
                };
                let projected: Vec<_> = points.iter().map(|&p| view.project(p)).collect();
                let expected: Vec<_> = projected
                    .windows(2)
                    .filter(|p| p[0].1 > 0 && p[1].1 > 0)
                    .map(|p| Line {
                        points: [p[0].0, p[1].0],
                        depth: (p[1].1 >> 2).clamp(0, 1249),
                        color: [100, 100, 200],
                    })
                    .collect();
                let mut effects = Effects::default();
                let mut random = Random::new(seed);
                let bytes =
                    effects.lightning(start, end, [100, 100, 200], &game.math, view, &mut random);
                assert_eq!(random.seed as i32, result[0], "case {case}");
                assert_eq!(random.calls as i32, result[1], "case {case}");
                assert_eq!(effects.lines, expected, "case {case} yaw {yaw}");
                assert_eq!(bytes, expected.len() as u32 * 20);
                assert!(effects.slots.iter().all(Option::is_none));
            }
        }
    }
}
