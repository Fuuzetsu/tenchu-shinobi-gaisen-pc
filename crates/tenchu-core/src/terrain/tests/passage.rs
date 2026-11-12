use super::*;
use crate::{oracle, random::Random};
use std::fmt::Write;

#[test]
#[ignore = "requires TENCHU_DISC, TENCHU_DECOMP and a C compiler; real mission sight rays against original terrain routines"]
fn mission_sight_passages_match_original_c() {
    let source = oracle::source("CONFLICT.C");
    let routines = [
        "long ComputeAreaLevel(",
        "long GetAreaMapLevel(",
        "VECTOR *GetAreaMapPassage(",
    ]
    .map(|signature| oracle::function(&source, signature).replace("long", "int32_t"))
    .join("\n");
    let harness = format!(
        r#"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
typedef uint32_t u32; typedef uint16_t u16;
typedef struct {{ int32_t vx,vy,vz; }} VECTOR;
typedef struct {{ int16_t vx,vy,vz; }} SVECTOR;
typedef struct {{ int16_t y,dy,x1,z1,x2,z2; uint16_t attribute,division; }} AreaNodeType;
typedef struct {{ int16_t y,n; void *index; int16_t x1,z1,x2,z2; }} NodeIndexType;
typedef NodeIndexType AreaMapType;
typedef struct {{ void *index; uint16_t array[6][4]; }} IndexArrayType;
enum area_level_mode_flag {{ AREA_LEVEL_DEFAULT=0, AREA_LEVEL_STEP_DOWN=1,
 AREA_LEVEL_RETURN_DELTA=2, AREA_LEVEL_ALLOW_DEEP=4, AREA_LEVEL_FIRST_HIT=8,
 AREA_LEVEL_REUSE_CACHED=16 }};
enum {{ MAP_ATTRIBUTE_UNRESOLVED=0x81, MAP_BUOYANT=2, MAP_RESULT_FINAL=0x2000,
 MAP_SLOPE_X=0x4000, MAP_SLOPE_Z=0x8000, AREA_INDEX_AXIS_SIZE=4,
 AREA_NODE_INDEX_NONE=0xffff, AREA_DIVISION_ALL=0xffff }};
#define LEVEL_NONE INT32_MIN
static NodeIndexType *FieldIndex;
static AreaNodeType *FieldArea;
static int32_t AreaMapLastY,FieldAttrib;
static VECTOR cv;
{routines}
int main(void) {{
 int rows;
 while(scanf("%d", &rows)==1) {{
  NodeIndexType *map=calloc(rows+1,sizeof(*map));
  for(int i=0;i<rows;i++) {{
   NodeIndexType *r=map+i;
   scanf("%hd%hd%hd%hd%hd%hd",&r->y,&r->n,&r->x1,&r->z1,&r->x2,&r->z2);
   AreaNodeType *nodes=calloc(abs(r->n),sizeof(*nodes)); r->index=nodes;
   if(r->n<0) {{
    IndexArrayType *lookup=calloc(1,sizeof(*lookup)); lookup->index=nodes;
    for(int j=0;j<21;j++) scanf("%hu",((uint16_t*)lookup->array)+j);
    r->index=lookup;
   }}
   for(int j=0;j<abs(r->n);j++) {{
    AreaNodeType *n=nodes+j;
    scanf("%hd%hd%hd%hd%hd%hd%hu%hu",&n->y,&n->dy,&n->x1,&n->z1,&n->x2,&n->z2,&n->attribute,&n->division);
   }}
  }}
  int cases; scanf("%d",&cases);
  FieldIndex=map; FieldArea=map->index; AreaMapLastY=INT32_MIN;
  for(int i=0;i<cases;i++) {{
   VECTOR p; SVECTOR d; int16_t n;
   scanf("%d%d%d%hd%hd%hd%hd",&p.vx,&p.vy,&p.vz,&d.vx,&d.vy,&d.vz,&n);
   VECTOR *hit=GetAreaMapPassage(map,&p,&d,n);
   if(hit) printf("1 %d %d %d\n",hit->vx,hit->vy,hit->vz);
   else puts("0");
  }}
  for(int i=0;i<rows;i++) {{
   if(map[i].n<0) free(((IndexArrayType*)map[i].index)->index);
   free(map[i].index);
  }}
  free(map);
 }}
}}
"#
    );
    let mut input = String::new();
    let mut expected = Vec::new();
    let mut random = Random::new(16);
    for stage in [0, 2, 9, 7] {
        let game = oracle::game(stage, 1);
        let map = &game.mission.terrain;
        writeln!(input, "{}", map.rows.len()).unwrap();
        let mut nodes = Vec::new();
        for row in &map.rows {
            writeln!(
                input,
                "{} {} {} {} {} {}",
                row.height,
                if row.lookup.is_some() {
                    -(row.count as i32)
                } else {
                    row.count as i32
                },
                row.bounds[0],
                row.bounds[1],
                row.bounds[2],
                row.bounds[3]
            )
            .unwrap();
            if row.lookup.is_some() {
                // Padded entries 16..20 represent the port's empty-edge policy.
                // This harness does not reproduce retail's out-of-bounds reads.
                let [x1, z1, x2, z2] = row.bounds.map(i32::from);
                let coordinate = |a, b, q| {
                    if q == 4 {
                        b
                    } else {
                        a + (b - a) * (2 * q + 1) / 8
                    }
                };
                for i in 0..21 {
                    let (qx, qz) = if i == 20 { (4, 4) } else { (i % 4, i / 4) };
                    write!(
                        input,
                        "{} ",
                        map.first_node(row, coordinate(x1, x2, qx), coordinate(z1, z2, qz))
                            .unwrap()
                    )
                    .unwrap();
                }
                input.push('\n');
            }
            for i in 0..row.count {
                let node = map.node(row.list + i * 16).unwrap();
                writeln!(
                    input,
                    "{} {} {} {} {} {} {} {}",
                    node.y,
                    node.dy,
                    node.bounds[0],
                    node.bounds[1],
                    node.bounds[2],
                    node.bounds[3],
                    node.attributes,
                    node.division
                )
                .unwrap();
                nodes.push(node);
            }
        }
        // Witness/body positions from the Mission 4 stealth-kill regression.
        // Compare retail's feet-to-head ray and the obstructed eye-to-body alternative.
        let corpse_rays = if stage == 9 {
            &[
                ([-22894, -7000, 155067], [-26882, -8600, 148810]),
                ([-22894, -8300, 155067], [-26882, -7300, 148810]),
                ([-35887, -7000, 155959], [-26828, -8600, 148990]),
                ([-35887, -8300, 155959], [-26828, -7300, 148990]),
            ][..]
        } else {
            &[][..]
        };
        let cases = 8192 + corpse_rays.len();
        writeln!(input, "{cases}").unwrap();
        let mut cursor = Cursor::new(map);
        let mut blocked = 0;
        for case in 0..cases {
            let (position, mut delta, limit) = if case >= 8192 {
                let (from, to) = corpse_rays[case - 8192];
                (
                    from.into(),
                    Position::from(to).wrapping_sub(from.into()),
                    500,
                )
            } else {
                let node = nodes[random.next15() as usize % nodes.len()];
                let [x1, z1, x2, z2] = node.bounds.map(|v| i32::from(v) * 10);
                (
                    Position::new(
                        [x1, (x1 + x2) / 2, x2][case % 3],
                        i32::from(node.y) * 10 - [900, 1500, 2100][case / 3 % 3],
                        [z1, (z1 + z2) / 2, z2][case / 9 % 3],
                    ),
                    Position::new(
                        random.next15() % 30001 - 15000,
                        random.next15() % 6001 - 3000,
                        random.next15() % 30001 - 15000,
                    ),
                    [300, 500][case % 2],
                )
            };
            let mut count = 1;
            while delta.iter().any(|v| v.abs() > limit) {
                delta = delta.map(|v| v >> 1);
                count *= 2;
            }
            writeln!(
                input,
                "{} {} {} {} {} {} {count}",
                position.x, position.y, position.z, delta.x, delta.y, delta.z
            )
            .unwrap();
            let result = passage(
                map,
                &mut cursor,
                position,
                delta.map(|v| v as i16).to_array(),
                count,
            )
            .unwrap();
            expected.push(if let Some(hit) = result {
                blocked += 1;
                vec![1, hit.x, hit.y, hit.z]
            } else {
                vec![0]
            });
        }
        assert!(
            blocked > 100 && blocked < cases - 100,
            "stage {stage}: {blocked}"
        );
        eprintln!(
            "M{}: {blocked} blocked, {} clear sight rays",
            game.mission.stage.order,
            cases - blocked
        );
    }
    let actual = oracle::run(&harness, &input);
    assert_eq!(actual.len(), expected.len());
    for (case, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(actual, &expected, "mission sight ray {case}");
    }
}
