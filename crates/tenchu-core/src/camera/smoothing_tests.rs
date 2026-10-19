use super::*;
use crate::oracle;
use std::fs;
use tenchu_assets::executable::Executable;

#[test]
#[ignore = "requires TENCHU_RETAIL_EXE, TENCHU_DECOMP and a C compiler; runs matching CAMERA.C as an oracle"]
fn smoothing_matches_original_c_with_narrowing_and_retained_history() {
    let exe =
        fs::read(std::env::var_os("TENCHU_RETAIL_EXE").expect("Set TENCHU_RETAIL_EXE")).unwrap();
    let math = Math {
        tables: Executable::new(&exe).unwrap().math_tables().unwrap(),
    };
    let source = oracle::source("CAMERA.C");
    let routine = oracle::function(&source, "static void MakeDifSub(");
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
typedef int32_t s32;
typedef int16_t s16;
typedef struct {{ s32 vx, vy, vz, pad; }} VECTOR;
typedef struct {{ s16 vx, vy, vz, pad; }} SVECTOR;
typedef struct {{ s16 div, ac, spd; SVECTOR bef; }} TMakeDifInfo;
#define FIXED_SHIFT 12
#define FIXED_ONE 4096
#define setVector(p,x,y,z) ((p)->vx=(x),(p)->vy=(y),(p)->vz=(z))
static int length[3], next_length;
static int GetVectorLength(int x, int y, int z) {{ return length[next_length++]; }}
{routine}
int main(void) {{
    VECTOR from, to, delta;
    TMakeDifInfo info;
    while (scanf("%d%d%d%d%d%d%hd%hd%hd%hd%hd%hd%d%d%d",
        &from.vx, &from.vy, &from.vz, &to.vx, &to.vy, &to.vz,
        &info.div, &info.ac, &info.spd, &info.bef.vx, &info.bef.vy, &info.bef.vz,
        &length[0], &length[1], &length[2]) == 15) {{
        next_length = 0;
        MakeDifSub(&from, &to, &delta, &info);
        printf("%d %d %d %d %d %d %d\n", delta.vx, delta.vy, delta.vz,
            info.spd, info.bef.vx, info.bef.vy, info.bef.vz);
    }}
}}
"#
    );
    let mut input = String::new();
    let mut expected = Vec::new();
    let mut random = crate::random::Random::new(123);
    for case in 0..512 {
        let mut next = || random.next15() * 13 - 200000;
        let from: [i32; 3] = std::array::from_fn(|_| next());
        let to = if case % 7 == 0 {
            from
        } else {
            std::array::from_fn(|_| next())
        };
        let mut smooth = Smooth {
            shift: case % 6,
            acceleration: next() as i16,
            speed: next() as i16,
            before: std::array::from_fn(|_| next() as i16),
        };
        let delta: [i32; 3] = std::array::from_fn(|i| to[i].wrapping_sub(from[i]));
        let lengths = [
            math.vector_distance(delta),
            math.vector_distance(delta.map(|v| v as i16 as i32)),
            math.vector_distance(smooth.before.map(i32::from)),
        ];
        let fields = from
            .into_iter()
            .chain(to)
            .chain([
                i32::from(smooth.shift),
                i32::from(smooth.acceleration),
                i32::from(smooth.speed),
            ])
            .chain(smooth.before.map(i32::from))
            .chain(lengths)
            .map(|v| v.to_string())
            .collect::<Vec<_>>();
        input.push_str(&fields.join(" "));
        input.push('\n');
        let result = smooth.delta(from, to, &math).unwrap();
        expected.push(
            result
                .into_iter()
                .chain([i32::from(smooth.speed)])
                .chain(smooth.before.map(i32::from))
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(oracle::run(&harness, &input), expected);
}
